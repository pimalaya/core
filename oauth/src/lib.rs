pub mod v2_0;

use std::result;
use thiserror::Error;

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    V2_0Error(#[from] v2_0::Error),
}
