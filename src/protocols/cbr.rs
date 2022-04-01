use crate::model::*;
use std::collections::{HashMap, HashSet};
use std::detect::__is_feature_detected::sha;
use crate::protocols::cbr::RoutableMessage::Ack;


const CBR_BEACON_PERIOD: u32 = 100;
const CBR_RETRY_PERIOD: u32 = 5 * 1000; /* 1 second */
const CBR_DROP_TIMEOUT: u32 = 1 * 1000;

type HintTable = HashMap<u32, u32>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutableMessage {
    Request(RequestMessage),
    Ack(u32),
}


#[derive(Debug, Clone)]
pub enum CBRMessage {
    Beacon(u32 /* from */),
    RoutingRequest((RoutableMessage, u32 /* destination */, u32 /* min_hint */, u32 /* origin_time */)),
}


pub async fn cbr_actor(my_id: u32, mut ctx: Context<CBRMessage>) {
    let mut last_transmission = ctx.current_step();
    let mut table = HintTable::from([
        (my_id, last_transmission),
    ]);
    ctx.send(MessageType::Comm(CBRMessage::Beacon(my_id)));

    log::info!("worker {} started", my_id);

    let mut messages_to_send = Vec::<(RoutableMessage, u32 /* destination */)>::new();
    let mut retries = HashMap::<u32, (RequestMessage, i32 /* last_sent */)>::new();
    while let Ok(event) = ctx.read_for(10).await {
        match event {
            Some(m) => match m.data {
                MessageType::Request(m) => {
                    retries.insert(m.id, (m.clone(), -(CBR_RETRY_PERIOD as i32)));
                }
                MessageType::Comm(m) => match m {
                    CBRMessage::RoutingRequest((rm, destination, min_hint, origin_time)) => {
                        if destination == my_id {
                            // This message has achieved its addressee
                            match rm {
                                RoutableMessage::Request(rm) => {
                                    ctx.send(MessageType::Request(rm.clone()));
                                    messages_to_send.push((RoutableMessage::Ack(rm.id), rm.from));
                                }
                                RoutableMessage::Ack(message_id) => {
                                    retries.remove(&message_id);
                                }
                            }
                        } else {
                            if ctx.current_step() - origin_time <= CBR_DROP_TIMEOUT {
                                // Consider broadcasting it
                                let mut shall_retransmit = !table.contains_key(&destination);
                                if !shall_retransmit {
                                    shall_retransmit |= (ctx.current_step() - table.get(&destination).unwrap()) <= min_hint
                                }
                                if shall_retransmit {
                                    messages_to_send.push((rm.clone(), destination));
                                }
                            }
                        }
                    }
                    CBRMessage::Beacon(from) => {
                        table.insert(from, ctx.current_step());
                    }
                },
            },
            None => {}
        }
        for (_, (rm, last_sent)) in retries.iter_mut() {
            if (ctx.current_step() as i32) - *last_sent >= CBR_RETRY_PERIOD as i32 {
                messages_to_send.push((RoutableMessage::Request(rm.clone()), rm.to));
            }
        }
        log::info!("Size of mq: {} (agent {}, retires: {}) [{:?}] \n RT:{:?}", messages_to_send.len(), my_id, retries.len(), messages_to_send, table);

        // Deduplicate messages
        messages_to_send.sort_by_key(|(rm, _)| match rm {
            RoutableMessage::Request(rm) => rm.id as i32,
            RoutableMessage::Ack(id) => -(*id as i32    ) - 1
        });
        messages_to_send.dedup();
        // Send all enqueued on this step messages
        for (msg, destination) in messages_to_send.iter() {
            ctx.send(MessageType::Comm(CBRMessage::RoutingRequest((
                msg.clone(),
                *destination,
                if table.contains_key(destination) {
                    ctx.current_step() - table.get(destination).unwrap()
                } else {
                    u32::MAX
                },
                ctx.current_step()
            ))));
            match msg {
                RoutableMessage::Request(rm) => {
                    if retries.contains_key(&rm.id) {
                        retries.entry(rm.id).and_modify(|(_, last_transmission)| { *last_transmission = ctx.current_step() as i32; });
                    }
                }
                RoutableMessage::Ack(_) => {}
            }
        }
        log::info!("Mq: {}", messages_to_send.len());
        messages_to_send.clear();

        if ctx.current_step() - last_transmission >= CBR_BEACON_PERIOD {
            last_transmission = ctx.current_step();
            ctx.send(MessageType::Comm(CBRMessage::Beacon(my_id)));
        }
    }


    log::info!("worker {} stopped", my_id);
}
