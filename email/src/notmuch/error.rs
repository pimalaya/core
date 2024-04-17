use std::{any::Any, result};

use thiserror::Error;

use crate::{AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open notmuch database")]
    OpenDatabaseError(#[source] notmuch::Error),
    #[error("cannot create notmuch query")]
    CreateQueryError(#[source] notmuch::Error),
    #[error("cannot execute notmuch query")]
    ExecuteQueryError(#[source] notmuch::Error),
    #[error("cannot close notmuch database")]
    CloseDatabaseError(#[source] notmuch::Error),
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
