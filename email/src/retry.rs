use std::{future::IntoFuture, time::Duration};

use tokio::time::{error::Elapsed, timeout, Timeout};

pub type Result<T> = std::result::Result<T, Elapsed>;

#[derive(Debug)]
pub enum RetryState<T> {
    Ok(T),
    Retry,
    TimedOut,
}

#[derive(Debug, Default)]
pub struct Retry {
    pub attempts: u8,
}

impl Retry {
    pub fn reset(&mut self) {
        self.attempts = 0;
    }

    pub fn timeout<F: IntoFuture>(&self, f: F) -> Timeout<F::IntoFuture> {
        timeout(Duration::from_secs(30), f)
    }

    pub fn next<T>(&mut self, res: Result<T>) -> RetryState<T> {
        match res.ok() {
            Some(res) => {
                return RetryState::Ok(res);
            }
            None if self.attempts < 3 => {
                self.attempts += 1;
                return RetryState::Retry;
            }
            None => {
                return RetryState::TimedOut;
            }
        }
    }
}
