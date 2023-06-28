use std::{collections::HashMap, fmt};

use crate::account::sync::Target;

use super::*;

pub type FolderName = String;
pub type FoldersName = HashSet<FolderName>;
pub type FolderSyncPatch = Vec<FolderSyncHunk>;
pub type FolderSyncPatches = HashMap<String, FolderSyncPatch>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSyncHunk {
    Create(FolderName, Target),
    Cache(FolderName, Target),
    Delete(FolderName, Target),
    Uncache(FolderName, Target),
}

impl fmt::Display for FolderSyncHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Create(folder, target) => write!(f, "Creating {target} folder {folder}"),
            Self::Cache(folder, target) => {
                write!(f, "Adding {target} folder {folder} to cache")
            }
            Self::Delete(folder, target) => write!(f, "Deleting {target} folder {folder}"),
            Self::Uncache(folder, target) => {
                write!(f, "Removing {target} folder {folder} from cache")
            }
        }
    }
}

impl FolderSyncHunk {
    pub fn folder(&self) -> &str {
        match self {
            Self::Create(folder, _) => folder.as_str(),
            Self::Cache(folder, _) => folder.as_str(),
            Self::Delete(folder, _) => folder.as_str(),
            Self::Uncache(folder, _) => folder.as_str(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSyncCacheHunk {
    Insert(FolderName, Target),
    Delete(FolderName, Target),
}
