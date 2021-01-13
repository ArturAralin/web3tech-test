use std::env;

#[derive(Clone)]
pub struct Config {
  pub max_concurrent_runs: usize,
  pub max_pending_runs: usize,
}

pub fn get_config() -> Config {
  let max_concurrent_runs = env::var("MAX_CONCURRENT_RUNS")
    .expect("MAX_CONCURRENT_RUNS must be provided")
    .parse::<usize>()
    .expect("MAX_CONCURRENT_RUNS must be an usize");

  let max_pending_runs = env::var("MAX_PENDING_RUNS")
    .expect("MAX_PENDING_RUNS must be provided")
    .parse::<usize>()
    .expect("MAX_PENDING_RUNS must be an usize");


  Config {
    max_concurrent_runs,
    max_pending_runs,
  }
}
