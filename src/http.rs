use std::convert::Infallible;
use std::sync::Arc;

use uuid::Uuid;
use warp::Filter;
use serde::{Serialize, Deserialize};

use tokio::sync::mpsc;

use super::Storage;
use super::worker;
use super::config::Config;

#[derive(Serialize)]
struct TaskBody {
  status: String,
  successful_responses_count: u32,
  sum: u32,
}

async fn get_tasks(tasks: Storage) -> Result<impl warp::Reply, Infallible> {
  if let Ok(tasks) = tasks.read() {
    let response = tasks.iter()
      .map(|(_, task)| {
        let status = match &task.state {
          worker::TaskState::Finished => "FINISHED",
          worker::TaskState::InProgress => "IN_PROGRESS",
        }.to_owned();

        TaskBody {
          status,
          successful_responses_count: task.successful_count,
          sum: task.acc
        }
      })
      .collect::<Vec<_>>();

    return Ok(warp::reply::json(&response))
  }

  Ok(warp::reply::json(&"Failed to ack read"))
}

#[derive(Deserialize)]
struct RunTaskBody {
  seconds: usize
}

#[derive(Serialize)]
struct RunTaskReply {
  id: Uuid
}

async fn run_task(
  config: Config,
  tasks: Storage,
  worker_tx: mpsc::UnboundedSender<worker::WorkerAction>,
  task: RunTaskBody
) -> Result<Box<dyn warp::Reply>, Infallible> {
  let id = Uuid::new_v4();

  let task = worker::Task::new(
    &id,
    config.max_concurrent_runs,
    task.seconds,
  );

  if let Ok(mut tasks) = tasks.write() {
    let active_count = tasks.iter().fold(0, |acc, (_, task)| match task.state {
      worker::TaskState::InProgress => acc + 1,
      worker::TaskState::Finished => acc,
    });

    if active_count >= config.max_pending_runs {
      return Ok(Box::new(warp::reply::with_status("429 Too Many Requests", warp::http::StatusCode::TOO_MANY_REQUESTS)));
    }

    task.start(worker_tx);
    tasks.insert(id.clone(), task);
  }

  Ok(Box::new(warp::reply::json(&RunTaskReply { id })))
}


pub fn runs(
  config: Config,
  tasks: &Storage,
  worker_tx: mpsc::UnboundedSender<worker::WorkerAction>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
  let get = warp::path("runs")
    .and(warp::get())
    .and(attach_tasks_hm(tasks.clone()))
    .and_then(get_tasks);

  let post = warp::path("runs")
    .and(warp::post())
    .and(attach_config(config))
    .and(attach_tasks_hm(tasks.clone()))
    .and(attach_worker_tx(worker_tx))
    .and(json_body())
    .and_then(run_task);

  get.or(post)
}


fn json_body() -> impl Filter<Extract = (RunTaskBody,), Error = warp::Rejection> + Clone {
  warp::body::json()
}


fn attach_tasks_hm(tasks: Storage) -> impl Filter<Extract = (Storage,), Error = Infallible> + Clone {
  warp::any().map(move || Arc::clone(&tasks))
}

fn attach_worker_tx(worker_tx: mpsc::UnboundedSender<worker::WorkerAction>) -> impl Filter<Extract = (mpsc::UnboundedSender<worker::WorkerAction>,), Error = Infallible> + Clone {
  warp::any().map(move || worker_tx.clone())
}

fn attach_config(config: Config) -> impl Filter<Extract = (Config,), Error = Infallible> + Clone {
  warp::any().map(move || config.clone())
}
