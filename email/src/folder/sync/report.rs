//! Module dedicated to email folders synchronization reporting.
//!
//! The core structure of this module is the [`FolderSyncReport`].

use crate::Error;

use super::{FolderSyncCacheHunk, FolderSyncHunk, FoldersName};

/// The folder synchronization report.
#[derive(Debug, Default)]
pub struct FolderSyncReport {
    /// The list of folders found during the synchronization process.
    pub names: FoldersName,

    /// The list of processed hunks associated with an optional
    /// error. Hunks that could not be processed are ignored.
    pub patch: Vec<(FolderSyncHunk, Option<Error>)>,

    /// The list of processed cache hunks associated with an optional
    /// error. Cache hunks that could not be processed are ignored.
    pub cache_patch: (Vec<FolderSyncCacheHunk>, Option<Error>),
}
