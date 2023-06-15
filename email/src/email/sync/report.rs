use super::{EmailSyncCacheHunk, EmailSyncHunk, Error};

#[derive(Debug, Default)]
pub struct EmailSyncReport {
    pub patch: Vec<(EmailSyncHunk, Option<Error>)>,
    pub cache_patch: (Vec<EmailSyncCacheHunk>, Option<Error>),
}
