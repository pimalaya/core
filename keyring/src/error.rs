//! # Error
//!
//! Module dedicated to keyring errors. It contains an [`Error`] enum
//! based on [`thiserror::Error`] and a type alias [`Result`].

use thiserror::Error;

use crate::native;

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build keyring entry using key `{1}`")]
    BuildEntryError(#[source] native::Error, String),
    #[error("cannot get secret from keyring matching `{1}`")]
    GetSecretError(#[source] native::Error, String),
    #[error("cannot find secret from keyring matching `{1}`")]
    FindSecretError(#[source] native::Error, String),
    #[error("cannot set secret from keyring matching `{1}`")]
    SetSecretError(#[source] native::Error, String),
    #[error("cannot delete secret from keyring matching `{1}`")]
    DeleteSecretError(#[source] native::Error, String),

    #[cfg(feature = "tokio")]
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}
