#[path = "../model/mod.rs"]
mod model;
use model::*;

async fn worker(id: u32, mut ctx: Context) {
    log::info!("worker {} started", id);
    while let Ok(_) = ctx.next_step().await {
        while let Some(m) = ctx.try_read() {
            log::info!("got message at {}: {:?}", id, m.data);
        }
        ctx.send(MessageType::Comm("hi!".to_string()));
    }
    log::info!("worker {} stopped", id);
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let (mut model, contexts) = Model::new(3);
    for i in 0..3 {
        model.conn.update(i, (i + 1) % 3, 1.0);
    }

    for (id, ctx) in contexts.into_iter().enumerate() {
        tokio::spawn(async move {
            worker(id as u32, ctx).await;
        });
    }
    for _ in 0..5 {
        model.step().await;
    }
    std::mem::drop(model);
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
