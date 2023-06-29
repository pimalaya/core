//! Module dedicated to emails synchronization reporting.
//!
//! The core structure of this module is the [`EmailSyncReport`].

use crate::Error;

use super::{EmailSyncCacheHunk, EmailSyncHunk};

/// The email synchronization report.
#[derive(Debug, Default)]
pub struct EmailSyncReport {
    /// The list of processed hunks associated with an optional
    /// error. Hunks that could not be processed are ignored.
    pub patch: Vec<(EmailSyncHunk, Option<Error>)>,

    /// The list of processed cache hunks associated with an optional
    /// error. Cache hunks that could not be processed are ignored.
    pub cache_patch: (Vec<EmailSyncCacheHunk>, Option<Error>),
}
