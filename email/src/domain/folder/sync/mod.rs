pub mod cache;
mod hunk;
mod patch;
mod report;

use log::error;
use std::{
    collections::{HashMap, HashSet},
    result,
};
use thiserror::Error;

pub use self::cache::Cache;
pub use self::hunk::{FolderSyncCacheHunk, FolderSyncHunk};
pub use self::patch::FolderSyncPatchManager;
pub use self::report::FolderSyncReport;
use crate::{account, backend};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] backend::maildir::Error),
    #[error(transparent)]
    BackendError(#[from] Box<backend::Error>),
}

pub type Result<T> = result::Result<T, Error>;

pub type FolderName = String;
pub type FoldersName = HashSet<FolderName>;
pub type FolderSyncPatch = Vec<FolderSyncHunk>;
pub type FolderSyncPatches = HashMap<String, FolderSyncPatch>;

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
