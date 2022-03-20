use std::collections::HashMap;

#[derive(Copy, Clone, Default, Debug)]
pub struct MessageStat {
    pub delivered: bool,
    pub steps: u32,
}

#[derive(Clone, Default, Debug)]
pub struct Stats {
    pub total: u32,
    pub delivered: u32,
    pub messages: HashMap<u32, MessageStat>,
}

impl Stats {
    pub fn all_delivered(&self) -> bool {
        return self.delivered == self.messages.len() as u32;
    }

    pub fn delivered(&mut self, id: u32, steps: u32) -> bool {
        let stat = self.messages.entry(id).or_default();
        if stat.delivered {
            return false;
        }
        self.delivered += 1;
        stat.delivered = true;
        stat.steps = steps;
        return true;
    }

    pub fn requested(&mut self, id: u32) {
        self.messages.entry(id).or_default();
    }

    pub fn on_message(&mut self) {
        self.total += 1;
    }
}
