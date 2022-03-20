use super::*;
use rand::prelude::SliceRandom;
use rand_distr::{Distribution, Uniform};
use std::collections::HashMap;
use tokio::sync::{mpsc, watch};

pub struct Model<T> {
    size: u32,
    step: u32,
    send: mpsc::UnboundedReceiver<(u32, MessageType<T>)>,
    recv: Vec<mpsc::UnboundedSender<Message<T>>>,
    buffer: HashMap<u32, Vec<Message<T>>>,
    done: mpsc::Receiver<()>,
    rng: rand::rngs::ThreadRng,
    pub conn: ConnMap,
    ticks: watch::Sender<u32>,
    messages: u32,
    pub stats: Stats,
}

fn vec_chan<T>(
    size: usize,
) -> (
    Vec<mpsc::UnboundedSender<T>>,
    Vec<mpsc::UnboundedReceiver<T>>,
) {
    let mut txs = Vec::with_capacity(size as usize);
    let mut rxs = Vec::with_capacity(size as usize);
    for _ in 0..size {
        let (tx, rx) = mpsc::unbounded_channel();
        txs.push(tx);
        rxs.push(rx);
    }
    (txs, rxs)
}

impl<T: Clone + core::fmt::Debug> Model<T> {
    pub fn new(size: u32) -> (Model<T>, Vec<Context<T>>) {
        let rng = rand::thread_rng();
        let recv = vec_chan(size as usize);
        let send = mpsc::unbounded_channel();
        let ticks = watch::channel(0).0;
        let mut contexts = Vec::with_capacity(size as usize);
        let done = mpsc::channel(size as usize);
        for (id, recv) in recv.1.into_iter().enumerate() {
            contexts.push(Context {
                send: send.0.clone(),
                recv,
                done: done.0.clone(),
                tick: ticks.subscribe(),
                step: 0,
                id: id as u32,
            });
        }
        let model = Model {
            size,
            step: 0,
            send: send.1,
            recv: recv.0,
            buffer: Default::default(),
            done: done.1,
            conn: Default::default(),
            rng,
            ticks,
            stats: Default::default(),
            messages: 0,
        };
        (model, contexts)
    }

    fn send_message(&mut self, from: u32, to: u32, data: &T) {
        if let Some(delay) = self.conn.get(from, to, &mut self.rng) {
            let at = self.step + delay;
            log::debug!(
                "staged message from {} to {} at {}: {:?}",
                from,
                to,
                at,
                data
            );
            self.buffer.entry(at).or_default().push(Message {
                data: MessageType::Comm(data.clone()),
                from,
                to,
            });
        }
    }

    fn broadcast(&mut self, from: u32, data: &T) {
        for to in 0..self.size {
            self.send_message(from, to, data);
        }
    }

    fn check_message(&mut self, sent: u32, data: RequestMessage) -> Result<(), anyhow::Error> {
        if data.to != sent {
            anyhow::bail!("wrong destination id");
        }
        if !self.stats.delivered(data.id, self.step - data.start) {
            log::info!("got duplicate message, id {}", data.id);
        } else {
            log::info!(
                "got message, id {} took {} steps",
                data.id,
                self.step - data.start
            );
        }
        Ok(())
    }

    fn process_message(&mut self, sent: u32, data: RequestMessage) {
        if let Err(err) = self.check_message(sent, data) {
            log::error!("wrong message {}", err);
        }
    }

    pub async fn step(&mut self) {
        for _ in 0..self.size {
            self.done.recv().await.unwrap();
        }
        self.step += 1;
        log::info!("step {}", self.step);
        while let Ok((id, data)) = self.send.try_recv() {
            match data {
                MessageType::Request(data) => self.process_message(id, data),
                MessageType::Comm(data) => self.broadcast(id, &data),
            }
        }
        if let Some(mut messages) = self.buffer.remove(&self.step) {
            messages.shuffle(&mut self.rng);
            for m in messages {
                self.stats.on_message();
                log::debug!("sending message from {} to {}: {:?}", m.from, m.to, m.data);
                self.recv[m.to as usize].send(m).unwrap();
            }
        }
        self.ticks.send(self.step).unwrap();
    }

    pub fn request_message(&mut self, from: u32, to: u32) {
        let id = self.messages;
        log::debug!("requested message id {} from {} to {}", id, from, to,);
        self.recv[from as usize]
            .send(Message {
                data: MessageType::Request(RequestMessage {
                    from,
                    to,
                    start: self.step,
                    id: id,
                }),
                from,
                to,
            })
            .unwrap();
        self.stats.requested(id);
        self.messages += 1
    }

    pub fn request_random(&mut self) {
        let between = Uniform::from(0..self.size);
        let from = between.sample(&mut self.rng);
        let mut to = between.sample(&mut self.rng);
        while to == from {
            to = between.sample(&mut self.rng);
        }
        self.request_message(from, to);
    }
}

pub struct Context<T> {
    send: mpsc::UnboundedSender<(u32, MessageType<T>)>,
    recv: mpsc::UnboundedReceiver<Message<T>>,
    tick: watch::Receiver<u32>,
    done: mpsc::Sender<()>,
    step: u32,
    id: u32,
}

impl<T> Context<T> {
    pub fn send(&self, data: MessageType<T>) {
        let _ = self.send.send((self.id, data));
    }

    async fn read_one(&mut self) -> Result<Option<Message<T>>, ()> {
        if let Some(message) = self.try_read() {
            return Ok(Some(message));
        }
        self.next_step().await?;
        return Ok(None);
    }

    pub async fn read_for(&mut self, steps: u32) -> Result<Option<Message<T>>, ()> {
        for _ in 0..steps {
            if let Some(m) = self.read_one().await? {
                return Ok(Some(m));
            }
        }
        return Ok(None);
    }

    pub async fn read(&mut self) -> Result<Message<T>, ()> {
        loop {
            if let Some(m) = self.read_one().await? {
                return Ok(m);
            }
        }
    }

    pub fn try_read(&mut self) -> Option<Message<T>> {
        self.recv.try_recv().ok()
    }

    pub async fn sleep(&mut self, steps: u32) -> Result<u32, ()> {
        let mut step = 0;
        for _ in 0..steps {
            step = self.next_step().await?;
        }
        return Ok(step);
    }

    pub async fn next_step(&mut self) -> Result<u32, ()> {
        let _ = self.done.send(()).await;
        self.tick.changed().await.map_err(|_| ())?;
        self.step = *self.tick.borrow();
        Ok(self.step)
    }

    pub fn current_step(&self) -> u32 {
        return self.step;
    }
}
