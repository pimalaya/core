use super::{EnvelopeSyncCacheHunk, EnvelopeSyncHunk, Error};

#[derive(Debug, Default)]
pub struct EnvelopeSyncReport {
    pub patch: Vec<(EnvelopeSyncHunk, Option<Error>)>,
    pub cache_patch: (Vec<EnvelopeSyncCacheHunk>, Option<Error>),
}
