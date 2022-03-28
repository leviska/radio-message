use std::borrow::Borrow;
use rand::{thread_rng, Rng};
use std::ops::Sub;

use crate::protocols::*;
use crate::scenarios::*;
use crate::model::*;

use euclid::*;

const STEPS_COUNT: i32 = 1000 /*ms*/ * 60 * 10;

// In meters
const FIELD_SIZE: f64 = 100.0;

// Velocity of order 0.001: Considering 1step~1ms, this makes a person traverse 1m in 1s.
const MIN_VELOCITY: f64 = 0.001 / 2.0;
const MAX_VELOCITY: f64 = 0.001 * 2.0;

const BASE_DELAY: f64 = 100.0;

const AGENTS_COUNT: u32 = 10;

#[derive(Copy, Clone)]
struct Agent {
    position: Point2D<f64, UnknownUnit>,
    destination: Point2D<f64, UnknownUnit>,
    velocity: f64, // Distance traversed by one person in 1 step
}

fn update_connections_via_positions<T: Clone + core::fmt::Debug>(model: &mut Model<T>, agents: &[Agent]) {
    for (i, a1) in agents.iter().enumerate() {
        for (j, a2) in agents.iter().enumerate() {
            let dst = (a1.position).distance_to(a2.position);
            let dst_frac = dst / ((2.0 * FIELD_SIZE.powf(2.)).sqrt());
            model.conn.update_both(i as u32, j as u32, (1. - dst_frac) as f32,
                                   (dst_frac * BASE_DELAY).ceil() as i32);
        }
    }
}

fn generate_agents(size: u32, rng: &mut impl Rng, field_random: &rand_distr::Uniform<f64>, speed_random: &rand_distr::Uniform<f64>) -> Vec<Agent> {
    let mut agents: Vec<Agent> = Vec::new();


    for _ in 0..size {
        agents.push(Agent {
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
    agents
}

async fn test_moving_random<T: Clone + core::fmt::Debug>(model: &mut Model<T>, agents_count: u32) {
    let mut rng = thread_rng();

    let field_random = rand_distr::Uniform::new(0.0, FIELD_SIZE);
    let speed_random = rand_distr::Uniform::new(MIN_VELOCITY, MAX_VELOCITY);

    let mut agents = generate_agents(agents_count, &mut rng, &field_random, &speed_random);

    update_connections_via_positions(model, &agents);

    send_batch(model, agents_count);


    // basically works like a timeout
    for _ in 0..STEPS_COUNT {
        // if all messages that were requested are delivered, break
        if model.stats.all_delivered() {
            break;
        }


        // Update connMap
        update_connections_via_positions(model, &agents);

        // also you can send additional messages, if you want, like
        // model.request_random();
        model.step().await;

        // Updating positions
        for (id, agent) in agents.iter_mut().enumerate() {
            log::info!("Position: {}\t{}", id, (agent.position - agent.destination).length());
            let mut steps_remaining = 1.0;
            loop {
                let remaining_dist = agent.destination.distance_to(agent.position);
                if remaining_dist > (steps_remaining * agent.velocity) {
                    // Nothing changes;

                    let mut direction = agent.destination.sub(agent.position);
                    direction = direction / direction.length() * steps_remaining * agent.velocity;
                    agent.position += direction;

                    break;
                } else {
                    steps_remaining -= remaining_dist / agent.velocity.borrow();
                    agent.position = agent.destination;
                    agent.destination = point2(
                        rng.sample(field_random),
                        rng.sample(field_random),
                    );
                    agent.velocity = rng.sample(speed_random);

                    log::info!("Achieved {}", id);
                }
            }
        }
    }

    log::info!("{:?}", model.stats);
}


#[tokio::test]
async fn test_moving() {
    let _ = env_logger::builder().try_init();
    {
        let mut model = generate_gossip_model(AGENTS_COUNT);
        test_moving_random::<GossipMessage>(&mut model, AGENTS_COUNT).await;
    }
    {
        let mut model = generate_dsdv_model(AGENTS_COUNT);
        test_moving_random::<DsdvMessage>(&mut model, AGENTS_COUNT).await;
    }
}