use std::collections::HashMap;

use rand_distr::Distribution;

const MIN_DELAY: i32 = 10;
const DEFAULT_DELAY: i32 = 50;
const DEVIATION: i32 = 10;

#[derive(Default, Clone)]
pub struct ConnMap {
    // probability that message will be delivered, [0; 1]
    pub prob: HashMap<(u32, u32), f32>,
    // shift in delay distribution
    pub delay: HashMap<(u32, u32), i32>,
}

impl ConnMap {
    pub fn update_prob(&mut self, from: u32, to: u32, prob: f32) {
        if prob == 0.0 {
            self.prob.remove(&(from, to));
        } else {
            *self.prob.entry((from, to)).or_default() = prob;
        }
    }

    pub fn update_prob_both(&mut self, first: u32, second: u32, prob: f32) {
        self.update_prob(first, second, prob);
        self.update_prob(second, first, prob);
    }

    pub fn update_delay(&mut self, from: u32, to: u32, delay: i32) {
        *self.delay.entry((from, to)).or_default() = delay;
    }

    pub fn update_delay_both(&mut self, first: u32, second: u32, delay: i32) {
        self.update_delay(first, second, delay);
        self.update_delay(second, first, delay);
    }

    pub fn update(&mut self, from: u32, to: u32, prob: f32, delay: i32) {
        self.update_prob(from, to, prob);
        self.update_delay(from, to, delay);
    }

    pub fn update_both(&mut self, first: u32, second: u32, prob: f32, delay: i32) {
        self.update(first, second, prob, delay);
        self.update(second, first, prob, delay);
    }

    pub fn prob(&self, from: u32, to: u32) -> f32 {
        self.prob.get(&(from, to)).cloned().unwrap_or_default()
    }

    pub fn delay(&self, from: u32, to: u32) -> i32 {
        self.delay.get(&(from, to)).cloned().unwrap_or_default()
    }

    pub fn test(&self, from: u32, to: u32, rng: &mut impl rand::Rng) -> bool {
        rng.gen_bool(self.prob(from, to) as f64)
    }

    pub fn get(&self, from: u32, to: u32, rng: &mut impl rand::Rng) -> Option<u32> {
        if !self.test(from, to, rng) {
            return None;
        }
        let rand = rand_distr::Normal::new(
            (DEFAULT_DELAY + self.delay(from, to)) as f32,
            DEVIATION as f32,
        )
        .unwrap();
        return Some(std::cmp::max(MIN_DELAY, rand.sample(rng) as i32) as u32);
    }
}
