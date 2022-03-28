use crate::model::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum GossipMessage {
    Request(RequestMessage),
    Ack(u32),
}

pub async fn gossip_actor(my_id: u32, mut ctx: Context<GossipMessage>) {
    log::info!("worker {} started", my_id);
    let mut history = HashMap::<u32, (GossipMessage, u32)>::default();
    while let Ok(event) = ctx.read_for(10).await {
        match event {
            Some(m) => match m.data {
                MessageType::Request(m) => {
                    let gm = GossipMessage::Request(m);
                    history.entry(m.id).or_insert((gm.clone(), 0));
                }
                MessageType::Comm(m) => match m {
                    GossipMessage::Request(m) => {
                        if m.to == my_id {
                            ctx.send(MessageType::Request(m));
                            history.entry(m.id).or_insert((GossipMessage::Ack(m.id), 0));
                        }
                        // if history[id] == Ack
                        if history
                            .get(&m.id)
                            .map(|x| {
                                matches!(x.0, GossipMessage::Ack(_)) && x.1 != ctx.current_step()
                            })
                            .unwrap_or_default()
                        {
                            history.entry(m.id).and_modify(|m| m.1 = ctx.current_step());
                            ctx.send(MessageType::Comm(GossipMessage::Ack(m.id)));
                        }
                    }
                    GossipMessage::Ack(id) => {
                        // if history[id] != Ack
                        if !history
                            .get(&id)
                            .map(|x| matches!(x.0, GossipMessage::Ack(_)))
                            .unwrap_or_default()
                        {
                            history
                                .entry(id)
                                .or_insert((GossipMessage::Ack(id), ctx.current_step()));
                            ctx.send(MessageType::Comm(GossipMessage::Ack(id)));
                        }
                    }
                },
            },
            None => {}
        }
        for (_, m) in history.iter_mut() {
            if matches!(m.0, GossipMessage::Request(_)) && ctx.current_step() - m.1 > 100 {
                ctx.send(MessageType::Comm(m.0.clone()));
                m.1 = ctx.current_step();
            }
        }
    }
    log::info!("worker {} stopped", my_id);
}
