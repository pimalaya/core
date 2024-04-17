//! # Email sync report
//!
//! Module dedicated to email synchronization reporting. The main
//! structure of this module is [`EmailSyncReport`].

use super::hunk::EmailSyncHunk;
use crate::AnyBoxedError;

/// The email synchronization report.
#[derive(Debug, Default)]
pub struct EmailSyncReport {
    /// The list of processed hunks associated with an optional error.
    pub patch: Vec<(EmailSyncHunk, Option<AnyBoxedError>)>,
}
