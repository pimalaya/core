use std::{io, result, string::FromUtf8Error};
use thiserror::Error;

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get standard input")]
    GetStdinError,
    #[error("cannot get exit status code of command: {0}")]
    GetExitStatusCodeNotAvailableError(String),
    #[error("command {0} returned non-zero exit status code {1}: {2}")]
    GetExitStatusCodeNonZeroError(String, i32, String),
    #[error("cannot parse command output as string")]
    ParseOutputAsUtf8StringError(#[source] FromUtf8Error),

    #[error(transparent)]
    IoError(#[from] io::Error),
}
