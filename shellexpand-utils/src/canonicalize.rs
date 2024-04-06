use log::debug;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

pub fn try_path(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    let canonicalized_path = path
        .canonicalize()
        .map_err(|err| Error::CanonicalizePathError(err, path.to_owned()))?;
    Ok(canonicalized_path)
}

pub fn path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    try_path(path).unwrap_or_else(|err| {
        debug!("{err:?}");
        path.to_owned()
    })
}
