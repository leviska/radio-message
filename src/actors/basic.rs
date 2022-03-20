use crate::model::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum GossipMessage {
    Request(RequestMessage),
    Ack(u32),
}

pub async fn gossip_actor(my_id: u32, mut ctx: Context<GossipMessage>) {
    log::info!("worker {} started", my_id);
    let mut history = HashMap::<u32, GossipMessage>::default();
    let mut last_broadcast = 0;
    loop {
        let event = ctx.read_for(10).await;
        match event {
            Ok(m) => {
                match m {
                    Some(m) => match m.data {
                        MessageType::Request(m) => {
                            let gm = GossipMessage::Request(m);
                            history.entry(m.id).or_insert(gm.clone());
                            ctx.send(MessageType::Comm(gm));
                        }
                        MessageType::Comm(m) => match m {
                            GossipMessage::Request(m) => {
                                if m.to == my_id {
                                    ctx.send(MessageType::Request(m));
                                    history.entry(m.id).or_insert(GossipMessage::Ack(m.id));
                                }
                                // if history[id] == Ack
                                if history
                                    .get(&m.id)
                                    .map(|x| matches!(x, GossipMessage::Ack(_)))
                                    .unwrap_or_default()
                                {
                                    ctx.send(MessageType::Comm(GossipMessage::Ack(m.id)));
                                } else {
                                    ctx.send(MessageType::Comm(GossipMessage::Request(m)));
                                }
                            }
                            GossipMessage::Ack(id) => {
                                // if history[id] != Ack
                                if !history
                                    .get(&id)
                                    .map(|x| matches!(x, GossipMessage::Ack(_)))
                                    .unwrap_or_default()
                                {
                                    ctx.send(MessageType::Comm(GossipMessage::Ack(id)));
                                }
                            }
                        },
                    },
                    None => {}
                }
                if ctx.current_step() - last_broadcast > 100 {
                    for (_, m) in history.iter() {
                        if matches!(m, GossipMessage::Request(_)) {
                            ctx.send(MessageType::Comm(m.clone()));
                        }
                    }
                    last_broadcast = ctx.current_step();
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    log::info!("worker {} stopped", my_id);
}
