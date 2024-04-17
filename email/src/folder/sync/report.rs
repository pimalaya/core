//! Module dedicated to email folders synchronization reporting.
//!
//! The core structure of this module is the [`FolderSyncReport`].

use super::hunk::{FolderSyncHunk, FoldersName};
use crate::AnyBoxedError;

/// The folder synchronization report.
#[derive(Debug, Default)]
pub struct FolderSyncReport {
    /// The list of folders found during the synchronization process.
    pub names: FoldersName,

    /// The list of processed hunks associated with an optional
    /// error. Hunks that could not be processed are ignored.
    pub patch: Vec<(FolderSyncHunk, Option<AnyBoxedError>)>,
}
