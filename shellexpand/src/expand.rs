use log::{debug, warn};
use std::{
    env::VarError,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot convert path {0:?} to string")]
    ConvertPathToStrError(PathBuf),
    #[error("cannot shell expand string {1}")]
    ExpandStrError(#[source] shellexpand_native::LookupError<VarError>, String),
}

pub fn try_path(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
    let path = path.as_ref();
    let path_str = path
        .to_str()
        .ok_or_else(|| Error::ConvertPathToStrError(path.to_owned()))?;
    let expanded_cow = shellexpand_native::full(path_str)
        .map_err(|err| Error::ExpandStrError(err, path_str.to_owned()))?;
    let expanded_path = PathBuf::from(expanded_cow.as_ref());
    Ok(expanded_path)
}

pub fn path(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    match try_path(path) {
        Ok(path) => path,
        Err(Error::ConvertPathToStrError(path)) => {
            warn!("cannot expand path {path:?}: cannot convert path to string");
            path
        }
        Err(Error::ExpandStrError(err, path)) => {
            warn!("{err}");
            debug!("{err:?}");
            PathBuf::from(path)
        }
    }
}

pub fn try_str(str: impl AsRef<str>) -> Result<String, Error> {
    let str = str.as_ref();
    let expanded_cow =
        shellexpand_native::full(str).map_err(|err| Error::ExpandStrError(err, str.to_owned()))?;
    let expanded_string = expanded_cow.to_string();
    Ok(expanded_string)
}

pub fn str(str: impl AsRef<str>) -> String {
    let str = str.as_ref();
    try_str(str).unwrap_or_else(|err| {
        warn!("{err}");
        debug!("{err:?}");
        str.to_owned()
    })
}
