pub mod cache;
mod hunk;
mod patch;
mod report;

use log::error;
use std::{collections::HashSet, result};
use thiserror::Error;

pub use self::cache::FolderSyncCache;
pub use self::hunk::{
    FolderName, FolderSyncCacheHunk, FolderSyncHunk, FolderSyncPatch, FolderSyncPatches,
    FoldersName,
};
pub use self::patch::{FolderSyncCachePatch, FolderSyncPatchManager};
pub use self::report::FolderSyncReport;
use crate::{account, backend};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    AccountConfigError(#[from] account::config::Error),
    #[error(transparent)]
    BackendError(#[from] backend::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] backend::maildir::Error),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum FolderSyncStrategy {
    #[default]
    All,
    Include(HashSet<String>),
    Exclude(HashSet<String>),
}

impl FolderSyncStrategy {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}
