use std::result;
use thiserror::Error;
use tokio::task::JoinError;

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build keyring entry using key `{1}`")]
    BuildEntryError(#[source] keyring_native::Error, String),
    #[error("cannot get secret from keyring matching `{1}`")]
    GetSecretError(#[source] keyring_native::Error, String),
    #[error("cannot find secret from keyring matching `{1}`")]
    FindSecretError(#[source] keyring_native::Error, String),
    #[error("cannot set secret from keyring matching `{1}`")]
    SetSecretError(#[source] keyring_native::Error, String),
    #[error("cannot delete secret from keyring matching `{1}`")]
    DeleteSecretError(#[source] keyring_native::Error, String),
    #[error("cannot build keyutils credentials using key {1}")]
    BuildCredentialsError(#[source] keyring_native::Error, String),

    #[error(transparent)]
    JoinError(#[from] JoinError),
}
