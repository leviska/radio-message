use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct ConnMap {
    // probability that message will be delivered, [0; 1]
    conn: HashMap<(u32, u32), f32>,
}

impl ConnMap {
    pub fn update(&mut self, from: u32, to: u32, prob: f32) {
        if prob == 0.0 {
            self.conn.remove(&(from, to));
        } else {
            *self.conn.entry((from, to)).or_default() = prob;
        }
    }

    pub fn update_both(&mut self, first: u32, second: u32, prob: f32) {
        self.update(first, second, prob);
        self.update(second, first, prob);
    }

    pub fn get(&self, from: u32, to: u32) -> f32 {
        self.conn.get(&(from, to)).cloned().unwrap_or_default()
    }

    pub fn test(&self, from: u32, to: u32, rng: &mut impl rand::Rng) -> bool {
        rng.gen_bool(self.get(from, to) as f64)
    }
}
