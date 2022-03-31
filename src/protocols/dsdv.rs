use crate::model::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingEntry {
    sequence_number: u32,
    next_hop: u32,
    metric: u64,
}

const DSDV_HEARTBEAT_PERIOD: u32 = 100;
const DSDV_RETRY_PERIOD: u32 = 1 * 1000; /* 1 second */

type RoutingTable = HashMap<u32, RoutingEntry>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutableMessage {
    Request(RequestMessage),
    Ack(u32),
}

#[derive(Debug, Clone)]
pub enum DSDVMessage {
    HeartBeat((RoutingTable, u32 /* from */)),
    RoutingRequest(
        (
            RoutableMessage,
            u32, /* rerouting_agent */
            u32, /* destination */
        ),
    ),
}

pub async fn dsdv_actor(my_id: u32, mut ctx: Context<DSDVMessage>) {
    let mut last_transmission = ctx.current_step();
    let mut table = RoutingTable::from([(
        my_id,
        RoutingEntry {
            metric: 0,
            next_hop: my_id,
            sequence_number: last_transmission,
        },
    )]);
    ctx.send(MessageType::Comm(DSDVMessage::HeartBeat((
        table.clone(),
        my_id,
    ))));

    log::info!("worker {} started", my_id);

    let mut messages_to_send = Vec::<(RoutableMessage, u32 /* destination */)>::new();
    let mut retries = HashMap::<u32, (RequestMessage, i32 /* last_sent */)>::new();

    while let Ok(event) = ctx.read_for(10).await {
        match event {
            Some(m) => match m.data {
                MessageType::Request(m) => {
                    retries.insert(m.id, (m.clone(), -(DSDV_RETRY_PERIOD as i32)));
                }
                MessageType::Comm(m) => match m {
                    DSDVMessage::RoutingRequest((rm, rerouting_agent, destination)) => {
                        // We are not supposed to see this message
                        if rerouting_agent == my_id {
                            if destination == my_id {
                                // This message has achieved its addressee
                                log::warn!(
                                    "Achieved its dest: {:?} {} {}",
                                    rm,
                                    rerouting_agent,
                                    destination
                                );
                                match rm {
                                    RoutableMessage::Request(rm) => {
                                        ctx.send(MessageType::Request(rm.clone()));
                                        messages_to_send
                                            .push((RoutableMessage::Ack(rm.id), rm.from));
                                    }
                                    RoutableMessage::Ack(message_id) => {
                                        retries.remove(&message_id);
                                    }
                                }
                            } else {
                                // Reroute it to someone else
                                messages_to_send.push((rm.clone(), destination));
                            }
                        }
                    }
                    DSDVMessage::HeartBeat((other_table, from)) => {
                        for (dst, entry) in other_table.iter() {
                            if !table.contains_key(dst)
                                || table.contains_key(dst)
                                    && table.get(dst).unwrap().sequence_number
                                        < entry.sequence_number
                            {
                                let mut new_entry = entry.clone();
                                new_entry.next_hop = from;
                                new_entry.metric = entry.metric + 1;
                                table.insert(dst.clone(), new_entry);
                            }
                        }
                    }
                },
            },
            None => {}
        }
        for (_, (rm, last_sent)) in retries.iter_mut() {
            if (ctx.current_step() as i32) - *last_sent >= DSDV_RETRY_PERIOD as i32 {
                messages_to_send.push((RoutableMessage::Request(rm.clone()), rm.to));
            }
        }
        log::info!(
            "Size of mq: {} (agent {}) [{:?}]",
            messages_to_send.len(),
            my_id,
            messages_to_send
        );

        // Deduplicate messages
        messages_to_send.sort_by_key(|(rm, _)| match rm {
            RoutableMessage::Request(rm) => rm.id,
            RoutableMessage::Ack(id) => *id,
        });
        messages_to_send.dedup();
        // Send all enqueued on this step messages
        let mut unsent_messages = Vec::<(RoutableMessage, u32 /* destination */)>::default();
        for (msg, destination) in messages_to_send.iter() {
            if table.contains_key(&destination) {
                ctx.send(MessageType::Comm(DSDVMessage::RoutingRequest((
                    msg.clone(),
                    table.get(&destination).unwrap().next_hop,
                    *destination,
                ))));
                match msg {
                    RoutableMessage::Request(rm) => {
                        if retries.contains_key(&rm.id) {
                            retries.entry(rm.id).and_modify(|(_, last_transmission)| {
                                *last_transmission = ctx.current_step() as i32;
                            });
                        }
                    }
                    RoutableMessage::Ack(_) => {}
                }
            } else {
                unsent_messages.push((*msg, *destination));
            }
        }
        messages_to_send.clear();
        assert!(messages_to_send.is_empty());
        messages_to_send.append(&mut unsent_messages);
        assert!(unsent_messages.is_empty());
        log::info!("Umq: {}", messages_to_send.len());
        if ctx.current_step() - last_transmission >= DSDV_HEARTBEAT_PERIOD {
            for (_, mut v) in table.iter_mut() {
                v.sequence_number = ctx.current_step();
            }
            ctx.send(MessageType::Comm(DSDVMessage::HeartBeat((
                table.clone(),
                my_id,
            ))));
            last_transmission = ctx.current_step();
        }
    }
    log::info!("worker {} stopped", my_id);
}
