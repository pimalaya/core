use crate::Error;

use super::{EmailSyncCacheHunk, EmailSyncHunk};

#[derive(Debug, Default)]
pub struct EmailSyncReport {
    pub patch: Vec<(EmailSyncHunk, Option<Error>)>,
    pub cache_patch: (Vec<EmailSyncCacheHunk>, Option<Error>),
}
