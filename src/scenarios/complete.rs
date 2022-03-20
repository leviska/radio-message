use crate::actors::*;
use crate::model::*;

const STEPS: i32 = 10000;

fn init_simple<T: Clone + core::fmt::Debug>(size: u32) -> (Model<T>, Vec<Context<T>>) {
    let (mut model, contexts) = Model::<T>::new(size);
    for i in 0..size {
        for j in i + 1..size {
            model.conn.update_both(i, j, 0.1, 0);
        }
    }
    (model, contexts)
}

fn send_batch<T: Clone + core::fmt::Debug>(model: &mut Model<T>, size: u32) {
    for _ in 0..size {
        model.request_random();
    }
}

#[tokio::test]
async fn gossip() {
    env_logger::init();

    let (mut model, contexts) = init_simple::<GossipMessage>(10);

    send_batch(&mut model, 10);

    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            gossip_actor(id as u32, ctx).await;
        });
    }

    for _ in 0..STEPS {
        if model.stats.all_delivered() {
            break;
        }
        model.step().await;
    }

    log::info!("{:?}", model.stats);
}
