use std::{io, path::PathBuf};

use advisory_lock::FileLockError;
use thiserror::Error;

/// Errors related to synchronization.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open sync lock file")]
    OpenLockFileSyncError(#[source] io::Error, PathBuf),
    #[error("cannot lock sync file")]
    LockFileSyncError(#[source] FileLockError, PathBuf),
    #[error("cannot unlock sync file")]
    UnlockFileSyncError(#[source] FileLockError, PathBuf),
    #[error("cannot get sync cache directory")]
    GetCacheDirectorySyncError,
}
