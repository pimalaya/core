use std::fmt;

use crate::backend::sync::Target;

use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSyncHunk {
    CreateFolder(FolderName, Target),
    CacheFolder(FolderName, Target),
    DeleteFolder(FolderName, Target),
    DeleteCachedFolder(FolderName, Target),
}

impl fmt::Display for FolderSyncHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateFolder(folder, target) => write!(f, "Creating {target} folder {folder}"),
            Self::CacheFolder(folder, target) => {
                write!(f, "Adding {target} folder {folder} to cache")
            }
            Self::DeleteFolder(folder, target) => write!(f, "Deleting {target} folder {folder}"),
            Self::DeleteCachedFolder(folder, target) => {
                write!(f, "Removing {target} folder {folder} from cache")
            }
        }
    }
}

impl FolderSyncHunk {
    pub fn folder(&self) -> &str {
        match self {
            Self::CreateFolder(folder, _) => folder.as_str(),
            Self::CacheFolder(folder, _) => folder.as_str(),
            Self::DeleteFolder(folder, _) => folder.as_str(),
            Self::DeleteCachedFolder(folder, _) => folder.as_str(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSyncCacheHunk {
    CreateFolder(FolderName, Target),
    DeleteFolder(FolderName, Target),
}
