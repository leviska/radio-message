use crate::protocols::*;
use crate::model::*;


fn init_simple<T: Clone + core::fmt::Debug>(size: u32) -> (Model<T>, Vec<Context<T>>) {
    let (model, contexts) = Model::<T>::new(size);
    (model, contexts)
}

pub fn generate_gossip_model(size: u32) -> Model<GossipMessage> {
    let (model, contexts) = init_simple::<GossipMessage>(size);
    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            gossip_actor(id as u32, ctx).await;
        });
    }
    return model;
}

pub fn generate_dsdv_model(size: u32) -> Model<DsdvMessage> {
    let (model, contexts) = init_simple::<DsdvMessage>(size);
    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            dsdv_actor(id as u32, ctx).await;
        });
    }
    return model;
}

pub fn send_batch<T: Clone + core::fmt::Debug>(model: &mut Model<T>, count: u32) {
    for _ in 0..count {
        model.request_random();
    }
}
