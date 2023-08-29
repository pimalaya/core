use std::path::{Path, PathBuf};
use thiserror::Error;

mod canonicalize;
mod expand;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ExpandError(#[from] expand::Error),
    #[error(transparent)]
    CanonicalizeError(#[from] canonicalize::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

pub fn try_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = expand::try_path(path)?;
    let path = canonicalize::try_path(path)?;
    Ok(path)
}

pub fn path(path: impl AsRef<Path>) -> PathBuf {
    canonicalize::path(expand::path(path))
}

pub fn try_str(str: impl AsRef<str>) -> Result<String> {
    let str = expand::try_str(str)?;
    Ok(str)
}

pub fn str(str: impl AsRef<str>) -> String {
    expand::str(str)
}
