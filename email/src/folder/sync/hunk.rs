//! Module dedicated to email folders synchronization hunk.
//!
//! The core structure of the module is the [`FolderSyncHunk`], which
//! represents a change in a patch.

use std::fmt;

use super::*;
use crate::sync::SyncDestination;

/// Alias for the folder name.
pub type FolderName = String;

/// Alias for the unique set of folder names.
pub type FoldersName = HashSet<FolderName>;

/// The folder synchronization hunk.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum FolderSyncHunk {
    /// The given folder name needs to be created to the given
    /// destination.
    Create(FolderName, SyncDestination),

    /// The given folder name needs to be added to the cache for the
    /// given destination.
    Cache(FolderName, SyncDestination),

    /// The given folder needs to be deleted from the given
    /// destination.
    Delete(FolderName, SyncDestination),

    /// The given folder needs to be removed from the cache for the
    /// given destination.
    Uncache(FolderName, SyncDestination),
}

impl FolderSyncHunk {
    pub fn is_left(&self) -> bool {
        match self {
            Self::Create(_, SyncDestination::Left) => true,
            Self::Cache(_, SyncDestination::Left) => true,
            Self::Delete(_, SyncDestination::Left) => true,
            Self::Uncache(_, SyncDestination::Left) => true,
            _ => false,
        }
    }

    pub fn is_right(&self) -> bool {
        match self {
            Self::Create(_, SyncDestination::Right) => true,
            Self::Cache(_, SyncDestination::Right) => true,
            Self::Delete(_, SyncDestination::Right) => true,
            Self::Uncache(_, SyncDestination::Right) => true,
            _ => false,
        }
    }

    pub fn folder(&self) -> &str {
        match self {
            Self::Create(folder, _) => folder.as_str(),
            Self::Cache(folder, _) => folder.as_str(),
            Self::Delete(folder, _) => folder.as_str(),
            Self::Uncache(folder, _) => folder.as_str(),
        }
    }
}

impl fmt::Display for FolderSyncHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create(folder, target) => write!(f, "Creating {target} folder {folder}"),
            Self::Cache(folder, target) => {
                write!(f, "Adding {target} folder {folder} to cache")
            }
            Self::Delete(folder, target) => write!(f, "Deleting {target} folder {folder}"),
            Self::Uncache(folder, target) => {
                write!(f, "Removing {target} folder {folder} from cache")
            }
        }
    }
}

/// The folder synchronization cache hunk.
///
/// Similar to the [`FolderSyncHunk`], except that this hunk is
/// specific to the cache (SQLite).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSyncCacheHunk {
    /// The given folder name needs to be added to the cache for the
    /// given destination.
    Insert(FolderName, SyncDestination),

    /// The given folder name needs to be removed from the cache for
    /// the given destination.
    Delete(FolderName, SyncDestination),
}
