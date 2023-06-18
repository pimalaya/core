pub mod cache;
mod hunk;
mod patch;
mod report;

use std::collections::HashSet;

#[doc(inline)]
pub use self::{
    cache::FolderSyncCache,
    hunk::{
        FolderName, FolderSyncCacheHunk, FolderSyncHunk, FolderSyncPatch, FolderSyncPatches,
        FoldersName,
    },
    patch::{FolderSyncCachePatch, FolderSyncPatchManager},
    report::FolderSyncReport,
};

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
