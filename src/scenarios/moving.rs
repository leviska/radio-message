use rand::{thread_rng, Rng};
use std::ops::Sub;

use envmnt::get_parse_or;

use crate::protocols::*;
use crate::scenarios::*;
use crate::model::*;

use euclid::*;
use rand_distr::num_traits::ToPrimitive;

const DEFAULT_STEPS_COUNT: u32 = 1000 * 60 * 10; /* 10 minutes */
const DEFAULT_AGENTS_COUNT: u32 = 10;
const DEFAULT_MESSAGES_COUNT: u32 = 10;
const DEFAULT_FIELD_SIZE: f64 = 100.0; /* 100 meters */
const DEFAULT_MIN_VELOCITY: f64 = 0.001 / 2.0; /* 0.5 m/sec */
const DEFAULT_MAX_VELOCITY: f64 = 0.001 * 2.0; /* 2.0 m/sec */
const DEFAULT_BASE_DELAY: f64 = 100.;
const DEFAULT_MAX_CONNECTION_RANGE: f64 = 30.; /* 30 m */
const DEFAULT_STARTUP_AWAIT: u32 = 5 * 1000; /* 5 seconds for a startup */

struct MovingModelParams {
    steps_count: u32,
    agents_count: u32,
    messages_count: u32,
    field_size: f64,
    min_velocity: f64,
    max_velocity: f64,
    base_delay: f64,
    max_connection_range: f64,
    startup_await: u32,
}


#[derive(Copy, Clone)]
struct Agent {
    position: Point2D<f64, UnknownUnit>,
    destination: Point2D<f64, UnknownUnit>,
    velocity: f64, // Distance traversed by one person in 1 step
}

fn update_connections_via_positions<T: Clone + core::fmt::Debug>(model: &mut Model<T>, agents: &[Agent], params: &MovingModelParams) {
    for (i, a1) in agents.iter().enumerate() {
        for (j, a2) in agents.iter().enumerate() {
            let dst = (a1.position).distance_to(a2.position);
            // let dst_frac = dst / ((2.0 * params.field_size.powf(2.)).sqrt());
            let delay = 0;
            let prob = if dst > params.max_connection_range { 0. } else { 1. };
            model.conn.update_both(i as u32, j as u32, prob.to_f32().unwrap(), delay);
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

async fn test_moving_random<T: Clone + core::fmt::Debug>(model: &mut Model<T>, agents_count: u32, params: &MovingModelParams) -> Stats {
    let mut rng = thread_rng();

    let field_random = rand_distr::Uniform::new(0.0, params.field_size);
    let speed_random = rand_distr::Uniform::new(params.min_velocity, params.max_velocity);

    let mut agents = generate_agents(agents_count, &mut rng, &field_random, &speed_random);

    update_connections_via_positions(model, &agents, &params);


    for _ in 0..params.startup_await {
        model.step().await;
    }

    send_batch(model, params.messages_count);

    // basically works like a timeout
    for _ in params.startup_await..params.steps_count {
        // if all messages that were requested are delivered, break
        if model.stats.all_delivered() {
            break;
        }
        log::warn!("{}", model.stats.delivered);


        // Update connMap
        update_connections_via_positions(model, &agents, &params);

        // also you can send additional messages, if you want, like
        // model.request_random();
        model.step().await;

        // Updating positions
        for (id, agent) in agents.iter_mut().enumerate() {
            log::debug!("Position: {}\t{}", id, (agent.position - agent.destination).length());
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
                    steps_remaining -= remaining_dist / agent.velocity;
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

    println!("{:?}", model.stats);

    return model.stats.clone();
}


#[tokio::test]
async fn test_moving() {
    let _ = env_logger::builder().try_init();

    let params = MovingModelParams {
        steps_count: get_parse_or("STEPS_COUNT", DEFAULT_STEPS_COUNT).unwrap(),
        agents_count: get_parse_or("AGENTS_COUNT", DEFAULT_AGENTS_COUNT).unwrap(),
        messages_count: get_parse_or("MESSAGES_COUNT", DEFAULT_MESSAGES_COUNT).unwrap(),
        field_size: get_parse_or("FIELD_SIZE", DEFAULT_FIELD_SIZE).unwrap(),
        min_velocity: get_parse_or("MIN_VELOCITY", DEFAULT_MIN_VELOCITY).unwrap(),
        max_velocity: get_parse_or("MAX_VELOCITY", DEFAULT_MAX_VELOCITY).unwrap(),
        base_delay: get_parse_or("BASE_DELAY", DEFAULT_BASE_DELAY).unwrap(),
        max_connection_range: get_parse_or("MAX_CONNECTION_RANGE", DEFAULT_MAX_CONNECTION_RANGE).unwrap(),
        startup_await: get_parse_or("STARTUP_AWAIT", DEFAULT_STARTUP_AWAIT).unwrap(),
    };

    {
        let mut model = generate_gossip_model(params.agents_count);
        let stats = test_moving_random::<GossipMessage>(&mut model, params.agents_count, &params).await;
        println!("Gossip protocol: delivered {}. Avg time: {}", stats.delivered, stats.avg_delivery_time());
    }
    {
        let mut model = generate_dsdv_model(params.agents_count);
        let stats = test_moving_random::<DSDVMessage>(&mut model, params.agents_count, &params).await;
        println!("Dsdv protocol: delivered {}. Avg time: {}", stats.delivered, stats.avg_delivery_time());
    }
}