#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

pub mod v2_0;

use thiserror::Error;

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    V2_0Error(#[from] v2_0::Error),
}
