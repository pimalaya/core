//! # Sync report
//!
//! Module dedicated to the synchronization report.

use crate::{email::sync::EmailSyncReport, folder::sync::FolderSyncReport};

/// The synchronization report.
///
/// A report is just a struct containing reports from the folders and
/// the emails synchronization.
#[derive(Debug, Default)]
pub struct SyncReport {
    /// The report of folders synchronization.
    pub folder: FolderSyncReport,

    /// The report of emails synchronization.
    pub email: EmailSyncReport,
}
