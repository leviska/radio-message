use crate::model::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum DsdvMessage {
    Request(RequestMessage),
    Ack(u32),
}

pub async fn dsdv_actor(my_id: u32, mut ctx: Context<DsdvMessage>) {
    log::info!("worker {} started", my_id);
    let mut history = HashMap::<u32, DsdvMessage>::default();
    let mut last_broadcast = 0;
    while let Ok(event) = ctx.read_for(10).await {
        match event {
            Some(m) => match m.data {
                MessageType::Request(m) => {
                    let gm = DsdvMessage::Request(m);
                    history.entry(m.id).or_insert(gm.clone());
                    ctx.send(MessageType::Comm(gm));
                }
                MessageType::Comm(m) => match m {
                    DsdvMessage::Request(m) => {
                        if m.to == my_id {
                            ctx.send(MessageType::Request(m));
                            history.entry(m.id).or_insert(DsdvMessage::Ack(m.id));
                        }
                        // if history[id] == Ack
                        if history
                            .get(&m.id)
                            .map(|x| matches!(x, DsdvMessage::Ack(_)))
                            .unwrap_or_default()
                        {
                            ctx.send(MessageType::Comm(DsdvMessage::Ack(m.id)));
                        } else {
                            ctx.send(MessageType::Comm(DsdvMessage::Request(m)));
                        }
                    }
                    DsdvMessage::Ack(id) => {
                        // if history[id] != Ack
                        if !history
                            .get(&id)
                            .map(|x| matches!(x, DsdvMessage::Ack(_)))
                            .unwrap_or_default()
                        {
                            ctx.send(MessageType::Comm(DsdvMessage::Ack(id)));
                        }
                    }
                },
            },
            None => {}
        }
        if ctx.current_step() - last_broadcast > 100 {
            for (_, m) in history.iter() {
                if matches!(m, DsdvMessage::Request(_)) {
                    ctx.send(MessageType::Comm(m.clone()));
                }
            }
            last_broadcast = ctx.current_step();
        }
    }
    log::info!("worker {} stopped", my_id);
}
