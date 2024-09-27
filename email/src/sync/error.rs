use std::{io, path::PathBuf, result};

use advisory_lock::FileLockError;
use thiserror::Error;

use crate::{email, folder, AnyBoxedError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open sync lock file at {1}")]
    OpenLockFileError(#[source] io::Error, PathBuf),
    #[error("cannot lock sync file at {1}")]
    LockFileError(#[source] FileLockError, PathBuf),
    #[error("cannot unlock sync file at {1}")]
    UnlockFileError(#[source] FileLockError, PathBuf),
    #[error("cannot get sync cache directory")]
    GetCacheDirectorySyncError,
    #[error("cannot sync folders")]
    SyncFoldersError(#[source] folder::Error),
    #[error("cannot expunge folders after sync")]
    ExpungeFoldersError(#[source] folder::Error),
    #[error("cannot sync emails")]
    SyncEmailsError(#[source] email::Error),
    #[error("cannot configure left sync context")]
    ConfigureLeftContextError(#[source] AnyBoxedError),
    #[error("cannot configure right sync context")]
    ConfigureRightContextError(#[source] AnyBoxedError),
    #[error("cannot sync: left context is not configured")]
    LeftContextNotConfiguredError(#[source] AnyBoxedError),
    #[error("cannot sync: right context is not configured")]
    RightContextNotConfiguredError(#[source] AnyBoxedError),
    #[error("cannot build sync pool context")]
    BuildSyncPoolContextError(#[source] AnyBoxedError),
}
