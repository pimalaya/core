use rusqlite;
use std::result;
use thiserror::Error;

use crate::{account, backend};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    BackendError(#[from] Box<backend::Error>),
}

pub type Result<T> = result::Result<T, Error>;
