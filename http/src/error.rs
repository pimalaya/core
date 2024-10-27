//! # Error
//!
//! Module dedicated to keyring errors. It contains an [`Error`] enum
//! based on [`thiserror::Error`] and a type alias [`Result`].

use thiserror::Error;
use ureq::http::Uri;

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("error while sending request to {1}")]
    SendBodyError(#[source] ureq::http::Error, Uri),
    #[error("error while sending GET request to {1}")]
    SendGetRequestError(#[source] ureq::Error, Uri),
    #[error("error while sending POST request to {1}")]
    SendPostRequestError(#[source] ureq::Error, Uri),
    #[error("error while sending request")]
    SendRequestError(#[source] ureq::Error),

    #[error(transparent)]
    UreqError(#[from] ureq::Error),
    #[error(transparent)]
    HttpError(#[from] ureq::http::Error),
    #[error(transparent)]
    UriError(#[from] ureq::http::uri::InvalidUri),
    #[cfg(feature = "tokio")]
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}
