mod worker;
mod http;
mod config;

use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub type Storage = Arc<RwLock<HashMap<Uuid, worker::Task>>>;

#[tokio::main]
async fn main() {
  let config = config::get_config();
  let tasks: Storage = Arc::new(RwLock::new(HashMap::new()));
  let worker_tx = worker::start(tasks.clone());

  warp::serve(http::runs(
    config,
    &tasks,
    worker_tx
  ))
  .run(([127, 0, 0, 1], 9900))
  .await;
}
