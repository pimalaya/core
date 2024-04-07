use std::{any::Any, result};
use thiserror::Error;
use tokio::task::JoinError;

use crate::{AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build thread pool context for thread {1}/{2}")]
    BuildContextError(#[source] AnyBoxedError, usize, usize),

    #[error(transparent)]
    JoinError(#[from] JoinError),
}

impl AnyError for Error {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<Error> for AnyBoxedError {
    fn from(err: Error) -> Self {
        Box::new(err)
    }
}
