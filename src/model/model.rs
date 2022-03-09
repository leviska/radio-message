use super::*;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tokio::sync::{mpsc, watch};

#[derive(Serialize, Deserialize)]
struct RequestMessage {
    from: u32,
    to: u32,
    start: u32,
    id: u32,
}

pub struct Model {
    size: u32,
    step: u32,
    send: Vec<mpsc::UnboundedReceiver<MessageType>>,
    recv: Vec<mpsc::UnboundedSender<Message>>,
    done: mpsc::Receiver<()>,
    rng: rand::rngs::ThreadRng,
    pub conn: ConnMap,
    ticks: watch::Sender<u32>,
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

impl Model {
    pub fn new(size: u32) -> (Model, Vec<Context>) {
        let rng = rand::thread_rng();
        let send = vec_chan(size as usize);
        let recv = vec_chan(size as usize);
        let ticks = watch::channel(0).0;
        let mut contexts = Vec::with_capacity(size as usize);
        let done = mpsc::channel(size as usize);
        for (id, (send, recv)) in send.0.into_iter().zip(recv.1.into_iter()).enumerate() {
            contexts.push(Context {
                send,
                recv,
                done: done.0.clone(),
                tick: ticks.subscribe(),
                id: id as u32,
            });
        }
        let model = Model {
            size,
            step: 0,
            send: send.1,
            recv: recv.0,
            done: done.1,
            conn: Default::default(),
            rng,
            ticks: ticks,
        };
        (model, contexts)
    }

    fn send_message(&mut self, from: u32, to: u32, data: &String) {
        if !self.conn.test(from, to, &mut self.rng) {
            return;
        }
        log::info!("sending message from {} to {}: {}", from, to, data);
        self.recv[to as usize]
            .send(Message {
                data: MessageType::Comm(data.clone()),
                from,
                to,
            })
            .unwrap();
    }

    fn broadcast(&mut self, from: u32, data: &String) {
        for to in 0..self.size {
            self.send_message(from, to, data);
        }
    }

    fn check_message(&self, sent: u32, data: String) -> Result<(), anyhow::Error> {
        let message: RequestMessage = serde_json::from_str(&data)?;
        if message.to != sent {
            anyhow::bail!("wrong destination id");
        }
        log::info!("got message, took {} steps", self.step - message.start);
        Ok(())
    }

    fn process_message(&self, sent: u32, data: String) {
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
        let mut messages = Vec::default();
        for (id, rx) in self.send.iter_mut().enumerate() {
            while let Ok(d) = rx.try_recv() {
                messages.push((id, d));
            }
        }
        for (id, data) in messages {
            match data {
                MessageType::Request(data) => self.process_message(id as u32, data),
                MessageType::Comm(data) => self.broadcast(id as u32, &data),
            }
        }
        self.ticks.send(self.step).unwrap();
    }
}

pub struct Context {
    send: mpsc::UnboundedSender<MessageType>,
    recv: mpsc::UnboundedReceiver<Message>,
    tick: watch::Receiver<u32>,
    done: mpsc::Sender<()>,
    id: u32,
}

impl Context {
    pub fn send(&self, data: MessageType) {
        let _ = self.send.send(data);
    }

    async fn read_one(&mut self) -> Result<Option<Message>, ()> {
        if let Some(message) = self.try_read() {
            return Ok(Some(message));
        }
        self.next_step().await?;
        return Ok(None);
    }

    pub async fn read_for(&mut self, steps: u32) -> Result<Option<Message>, ()> {
        for _ in 0..steps {
            if let Some(m) = self.read_one().await? {
                return Ok(Some(m));
            }
        }
        return Ok(None);
    }

    pub async fn read(&mut self) -> Result<Message, ()> {
        loop {
            if let Some(m) = self.read_one().await? {
                return Ok(m);
            }
        }
    }

    pub fn try_read(&mut self) -> Option<Message> {
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
        Ok(*self.tick.borrow())
    }
}
