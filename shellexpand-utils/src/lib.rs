pub mod canonicalize;
mod error;
pub mod expand;

use std::path::{Path, PathBuf};

#[doc(inline)]
pub use crate::error::{Error, Result};

pub fn try_shellexpand_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = expand::try_path(path)?;
    let path = canonicalize::try_path(path)?;
    Ok(path)
}

pub fn shellexpand_path(path: impl AsRef<Path>) -> PathBuf {
    canonicalize::path(expand::path(path))
}

pub fn try_shellexpand_str(str: impl AsRef<str>) -> Result<String> {
    let str = expand::try_str(str)?;
    Ok(str)
}

pub fn shellexpand_str(str: impl AsRef<str>) -> String {
    expand::str(str)
}
