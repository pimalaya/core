//! # Error
//!
//! Module dedicated to secret errors. It contains an [`Error`] enum
//! based on [`thiserror::Error`] and a type alias [`Result`].

use thiserror::Error;

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get empty secret")]
    GetEmptySecretError,
    #[cfg(feature = "command")]
    #[error("cannot get secret from command")]
    GetSecretFromCommand(#[source] process::Error),
    #[cfg(feature = "command")]
    #[error("cannot get secret from command: empty output")]
    GetSecretFromCommandEmptyOutputError,

    #[cfg(feature = "keyring")]
    #[error(transparent)]
    KeyringError(#[from] keyring::Error),
}
