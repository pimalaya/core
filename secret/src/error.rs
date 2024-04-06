use std::result;
use thiserror::Error;

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get secret: secret is not defined")]
    GetUndefinedSecretError,
    #[cfg(feature = "command")]
    #[error("cannot get secret from command")]
    GetSecretFromCommand(#[source] process::Error),
    #[cfg(feature = "command")]
    #[error("cannot get secret from command: output is empty")]
    GetSecretFromCommandEmptyOutputError,
    #[cfg(feature = "keyring")]
    #[error("error while using secret from keyring")]
    KeyringError(#[source] keyring::Error),
}
