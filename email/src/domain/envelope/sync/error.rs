use rusqlite;
use std::result;
use thiserror::Error;

use crate::{account, backend, email};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find email by internal id {0}")]
    FindEmailError(String),
    #[error("cannot find email by internal id {0}")]
    LockConnectionPoolCursorError(String),
    #[error("cannot find email by internal id {0}")]
    FindConnectionByCursorError(usize),
    #[error("cannot find email by internal id {0}")]
    LockConnectionError(String),

    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    BackendError(#[from] Box<backend::Error>),
}

pub type Result<T> = result::Result<T, Error>;
