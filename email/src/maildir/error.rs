use std::{any::Any, path::PathBuf, result};

use thiserror::Error;

use crate::{AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("error while checking maildir configuration")]
    CheckConfigurationInvalidPathError(#[source] shellexpand_utils::Error),
    #[error("error while checking up current maildir directory")]
    CheckUpCurrentDirectoryError(#[source] maildirs::Error),
    #[error("cannot create maildir folder structure at {0}")]
    CreateFolderStructureError(#[source] maildirs::Error, PathBuf),

    #[error(transparent)]
    ExpandPathError(#[from] shellexpand_utils::Error),
    #[error(transparent)]
    MaildirError(#[from] maildirs::Error),
}

impl AnyError for Error {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<Error> for AnyBoxedError {
    fn from(err: Error) -> Self {
        Box::new(err)
    }
}
