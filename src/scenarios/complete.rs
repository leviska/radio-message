use rand::thread_rng;

use crate::model::*;
use crate::protocols::*;
use crate::scenarios::*;

const STEPS: i32 = 10000;

fn init_simple<T: Clone + core::fmt::Debug, R>(
    size: u32,
    rng: R,
) -> (Model<T, R>, Vec<Context<T>>) {
    let (mut model, contexts) = Model::new(size, rng);
    for i in 0..size {
        for j in i + 1..size {
            model.conn.update_both(i, j, 0.1, 0);
        }
    }
    (model, contexts)
}

#[tokio::test]
async fn gossip() {
    let _ = env_logger::builder().try_init();

    const SIZE: u32 = 10;

    let (mut model, contexts) = init_simple(SIZE, thread_rng());
    send_batch(&mut model, SIZE);

    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            gossip_actor(id as u32, ctx).await;
        });
    }

    // basically works like a timeout
    for _ in 0..STEPS {
        // if all messages that were requested are delivered, break
        if model.stats.all_delivered() {
            break;
        }

        //you can safely change conn params here, like
        /*
        for i in 0..SIZE {
            for j in i + 1..SIZE {
                model.conn.update_both(i, j, 0.2, step%50);
            }
        }
        */

        // also you can send additional messages, if you want, like
        // model.request_random();

        model.step().await;
    }

    log::info!("{:?}", model.stats);
}
