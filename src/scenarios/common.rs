use rand::Rng;

use crate::model::*;
use crate::protocols::*;

pub fn generate_gossip_model<R>(size: u32, rng: R) -> Model<GossipMessage, R> {
    let (model, contexts) = Model::new(size, rng);
    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            gossip_actor(id as u32, ctx).await;
        });
    }
    return model;
}

pub fn generate_dsdv_model<R>(size: u32, rng: R) -> Model<DSDVMessage, R> {
    let (model, contexts) = Model::new(size, rng);
    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            dsdv_actor(id as u32, ctx).await;
        });
    }
    return model;
}

pub fn generate_cbr_model<R>(size: u32, rng: R) -> Model<CBRMessage, R> {
    let (model, contexts) = Model::new(size, rng);
    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            cbr_actor(id as u32, ctx).await;
        });
    }
    return model;
}

pub fn send_batch<T: Clone + core::fmt::Debug, R: Rng>(model: &mut Model<T, R>, count: u32) {
    for _ in 0..count {
        model.request_random();
    }
}
