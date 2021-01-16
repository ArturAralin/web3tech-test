use serde_json;
use reqwest;
use uuid::Uuid;
use tokio::sync::mpsc::{channel, Sender, UnboundedSender, unbounded_channel};
use tokio::time::Duration;

use super::Storage;

#[derive(Debug)]
pub enum TaskState {
  InProgress,
  Finished,
}

#[derive(Debug)]
pub enum QueryResult {
  Success(u32),
  TimeOut,
  InternalError,
  TooManyRequests
}

pub struct TaskContext {
  id: Uuid,
  max_concurrent: usize,
  current_concurrent: usize,
  max_concurrent_reached: usize,
  duration: usize,
}

#[derive(Debug)]
pub struct Task {
  pub id: Uuid,
  pub state: TaskState,
  pub successful_count: u32,
  pub acc: u32,

  max_concurrent: usize,
  duration: usize,
}

impl Task {
  pub fn new(
    id: &Uuid,
    max_concurrent: usize,
    duration: usize
  ) -> Self {
    Self {
      id: *id,
      state: TaskState::InProgress,
      max_concurrent,
      duration,
      acc: 0,
      successful_count: 0,
    }
  }

  async fn handle_query(request_id: &Uuid) -> Result<QueryResult, reqwest::Error> {
    let response = reqwest::Client::new()
      .get("http://faulty-server-htz-nbg1-1.wvservices.exchange:8080")
      .header("X-Run-Id", format!("{}", request_id))
      .send()
      .await?;

    let body: serde_json::Value = response.json().await?;

    if body["value"].is_u64() {
      let val: u32 = serde_json::from_value(body["value"].clone()).unwrap();

      return Ok(QueryResult::Success(val))
    }

    if body["error"].is_string() {
      let error: String = serde_json::from_value(body["error"].clone()).unwrap();

      let result = match error.as_str() {
        "Timed out" => QueryResult::TimeOut,
        "Internal server error" => QueryResult::InternalError,
        "Too many concurrent requests" => QueryResult::TooManyRequests,
        _ => {
          unreachable!();
        }
      };

      return Ok(result);
    }

    unreachable!();
  }

  fn send_query(
    id: Uuid,
    mut tx: Sender<QueryResult>
  ) {
    tokio::spawn(async move {
      match Self::handle_query(&id).await {
        Ok(result) => {
          if let Err(_) = tx.send(result).await {
            // Ignore channel close
          };
        },
        Err(e) => {
          eprintln!("Request error {:?}", e);
        }
      }
    });
  }

  pub fn start(&self, worker_tx: UnboundedSender<WorkerAction>) {
    let mut task_ctx = TaskContext {
      id: self.id.clone(),
      duration: self.duration.clone(),
      current_concurrent: 0,
      max_concurrent_reached: self.max_concurrent / 2,
      max_concurrent: self.max_concurrent.clone(),
    };

    tokio::spawn(async move {
      println!("[{}] Task has started", task_ctx.id);
      let mut timer = tokio::time::delay_for(Duration::from_secs(task_ctx.duration as u64));
      let (query_tx, mut query_rx) = channel::<QueryResult>(1000);

      loop {
        tokio::select! {
          _ = &mut timer, if timer.is_elapsed() => {
            if let Err(e) = worker_tx.send(WorkerAction::TaskFinished(task_ctx.id)) {
              eprintln!("Failed to send TaskFinished: {}", e);
            }

            return;
          }

          Some(query_result) = query_rx.recv() => {
            match query_result {
              QueryResult::Success(_) | QueryResult::InternalError | QueryResult::TimeOut => {
                if task_ctx.max_concurrent != task_ctx.max_concurrent_reached {
                  task_ctx.max_concurrent_reached += 1;
                }
              },
              QueryResult::TooManyRequests => {},
            }

            task_ctx.current_concurrent -= 1;

            if let Err(e) = worker_tx.send(WorkerAction::QueryResult(task_ctx.id.clone(), query_result)) {
              eprintln!("Failed to send TaskFinished: {}", e);
            }
          }

          _ = tokio::time::delay_for(Duration::from_millis(50)) => {
            if task_ctx.current_concurrent < task_ctx.max_concurrent_reached {
              task_ctx.current_concurrent += 1;
              Self::send_query(
                task_ctx.id.clone(),
                query_tx.clone()
              );
            }
          }
        }
      }
    });
  }
}

#[derive(Debug)]
pub enum WorkerAction {
  TaskFinished(Uuid),
  QueryResult(Uuid, QueryResult),
}

// upd: Ещё немного подумав, я сообразил, что можно было обойтись одним
// лупом с селектом и двумя, а то и одним каналом
// тем самым избавиться от ненужного delay_for на 25ms
pub fn start(tasks: Storage) -> UnboundedSender<WorkerAction> {
  let (worker_tr, mut worker_rx) = unbounded_channel::<WorkerAction>();

  tokio::spawn(async move {
    loop {
      tokio::select! {
        Some(action) = worker_rx.recv() => {
          match action {
            WorkerAction::TaskFinished(task_id) => {
              if let Ok(mut tasks) = tasks.write() {
                if let Some(task) = tasks.get_mut(&task_id) {
                  println!("[{}] Finished", task_id);
                  task.state = TaskState::Finished;
                }
              }
            },
            WorkerAction::QueryResult(task_id, query_result) => {
              println!("[{}] {:?}", task_id, query_result);
              if let QueryResult::Success(value) = query_result {
                if let Ok(mut tasks) = tasks.write() {
                  if let Some(task) = tasks.get_mut(&task_id) {
                    task.acc += value;
                    task.successful_count += 1;
                  }
                }
              }
            },
          }
        }
        else => {}
      }
    }
  });

  worker_tr
}
