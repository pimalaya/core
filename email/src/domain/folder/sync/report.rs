use super::*;

#[derive(Debug, Default)]
pub struct FolderSyncReport {
    pub folders: FoldersName,
    pub patch: Vec<(FolderSyncHunk, Option<Error>)>,
    pub cache_patch: (Vec<FolderSyncCacheHunk>, Option<Error>),
}
