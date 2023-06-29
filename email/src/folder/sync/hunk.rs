//! Module dedicated to email folders synchronization hunk.
//!
//! The core structure of the module is the [`FolderSyncHunk`], which
//! represents a change in a patch.

use std::fmt;

use crate::account::sync::Target;

use super::*;

/// Alias for the folder name.
pub type FolderName = String;

/// Alias for the unique set of folder names.
pub type FoldersName = HashSet<FolderName>;

/// The folder synchronization hunk.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSyncHunk {
    /// The given folder name needs to be created to the given
    /// destination.
    Create(FolderName, Target),

    /// The given folder name needs to be added to the cache for the
    /// given destination.
    Cache(FolderName, Target),

    /// The given folder needs to be deleted from the given
    /// destination.
    Delete(FolderName, Target),

    /// The given folder needs to be removed from the cache for the
    /// given destination.
    Uncache(FolderName, Target),
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

impl FolderSyncHunk {
    pub fn folder(&self) -> &str {
        match self {
            Self::Create(folder, _) => folder.as_str(),
            Self::Cache(folder, _) => folder.as_str(),
            Self::Delete(folder, _) => folder.as_str(),
            Self::Uncache(folder, _) => folder.as_str(),
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
    Insert(FolderName, Target),

    /// The given folder name needs to be removed from the cache for
    /// the given destination.
    Delete(FolderName, Target),
}
