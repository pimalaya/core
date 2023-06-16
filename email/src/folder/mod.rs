//! Module dedicated to folders (mailboxes).

pub mod sync;

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

pub use self::sync::{
    FolderSyncCache, FolderSyncCacheHunk, FolderSyncCachePatch, FolderSyncHunk, FolderSyncPatch,
    FolderSyncPatchManager, FolderSyncPatches, FolderSyncStrategy,
};

/// Represents the list of folders.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Folders(Vec<Folder>);

impl Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Folders {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<Folder> for Folders {
    fn from_iter<T: IntoIterator<Item = Folder>>(iter: T) -> Self {
        let mut folders = Folders::default();
        folders.extend(iter);
        folders
    }
}

/// Represents the folder.
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Folder {
    /// Represents the folder hierarchie delimiter.
    pub delim: String,
    /// Represents the folder name.
    pub name: String,
    /// Represents the folder description.
    pub desc: String,
}

impl PartialEq for Folder {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl fmt::Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}
