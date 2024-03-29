use std::{env::VarError, io, path::PathBuf};

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("cannot convert path {0:?} to string")]
    ConvertPathToStrError(PathBuf),
    #[error("cannot shell expand string {1}")]
    ExpandStrError(#[source] shellexpand::LookupError<VarError>, String),
    #[error("cannot canonicalize path {1:?}")]
    CanonicalizePathError(#[source] io::Error, PathBuf),
}
