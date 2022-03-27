use std::borrow::Borrow;
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use std::ops::Sub;

use crate::actors::*;
use crate::model::*;

use euclid::*;

const STEPS_COUNT: i32 = 1000 /*ms*/ * 60 * 10;

// In meters
const FIELD_SIZE: f64 = 100.0;

// Velocity of order 0.001: Considering 1step~1ms, this makes a person traverse 1m in 1s.
const MIN_VELOCITY: f64 = 0.001 / 2.0;
const MAX_VELOCITY: f64 = 0.001 * 2.0;

const BASE_DELAY: f64 = 100.0;

const ACTORS_COUNT: u32 = 10;

#[derive(Copy, Clone)]
struct Agent {
    position: Point2D<f64, UnknownUnit>,
    destination: Point2D<f64, UnknownUnit>,
    velocity: f64, // Distance traversed by one person in 1 step
}


fn init_simple<T: Clone + core::fmt::Debug>(
    size: u32,
) -> (Model<T>, Vec<Context<T>>) {
    let (mut model, contexts) = Model::<T>::new(size);
    for i in 0..size {
        // for j in i + 1..size {
        //     model.conn.update_both(i, j, 1., 0);
        // }
        model.conn.update(i, (i + 1) % size, 1., 0);
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
    let _ = env_logger::builder().try_init();


    let mut actors: Vec<Agent> = Vec::new();

    let mut rng = thread_rng();
    let field_random = rand_distr::Uniform::new(0.0, FIELD_SIZE);
    let speed_random = rand_distr::Uniform::new(MIN_VELOCITY, MAX_VELOCITY);

    for _ in 0..ACTORS_COUNT {
        actors.push(Agent {
            position: point2(
                rng.sample(field_random),
                rng.sample(field_random),
            ),
            destination: point2(
                rng.sample(field_random),
                rng.sample(field_random),
            ),
            velocity: rng.sample(speed_random),
        });
    }

    let (mut model, contexts) = init_simple::<GossipMessage>(ACTORS_COUNT);
    send_batch(&mut model, ACTORS_COUNT);

    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            gossip_actor(id as u32, ctx).await;
        });
    }

    // basically works like a timeout
    for _ in 0..STEPS_COUNT {
        // if all messages that were requested are delivered, break
        if model.stats.all_delivered() {
            break;
        }

        // Update connMap
        for (i, a1) in actors.iter().enumerate() {
            for (j, a2) in actors.iter().enumerate() {
                let dst = (a1.position).distance_to(a2.position);
                let dst_frac = dst / ((2.0 * FIELD_SIZE.powf(2.)).sqrt());
                model.conn.update_both(i as u32, j as u32, (1. - dst_frac) as f32, (dst_frac * BASE_DELAY).ceil() as i32);
            }
        }

        // also you can send additional messages, if you want, like
        // model.request_random();
        model.step().await;

        // Updating positions
        for (id, actor) in actors.iter_mut().enumerate() {
            let mut steps_remaining = 1.0;
            loop {
                let remaining_dist = actor.destination.distance_to(actor.position);
                if remaining_dist > (steps_remaining * actor.velocity) {
                    // Nothing changes;

                    let mut direction = actor.destination.sub(actor.position);
                    direction = direction / direction.length() * steps_remaining * actor.velocity;
                    actor.position += direction;

                    break;
                } else {
                    steps_remaining -= remaining_dist / actor.velocity.borrow();
                    actor.position = actor.destination;
                    actor.destination = point2(
                        rng.sample(field_random),
                        rng.sample(field_random),
                    );
                    actor.velocity = rng.sample(speed_random);

                    log::info!("Achieved {}", id);
                }
            }
        }
    }

    log::info!("{:?}", model.stats);
}
