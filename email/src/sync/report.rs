//! # Sync report
//!
//! Module dedicated to synchronization reporting. The main structure
//! of thi module is [`SyncReport`].

use crate::{email::sync::report::EmailSyncReport, folder::sync::report::FolderSyncReport};

/// The synchronization report.
///
/// A report is just a struct containing reports from the folders and
/// the emails synchronization.
#[derive(Debug, Default)]
pub struct SyncReport {
    /// The report of folder synchronization.
    pub folder: FolderSyncReport,

    /// The report of email synchronization.
    pub email: EmailSyncReport,
}
