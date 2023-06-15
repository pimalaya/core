use std::fmt;

use crate::{
    backend::sync::{Source, Target},
    folder::sync::FolderName,
    Envelope,
};

pub type Id = String;
pub type RefreshSourceCache = bool;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum EnvelopeSyncHunk {
    GetThenCache(FolderName, Id, Source),
    CopyThenCache(FolderName, Envelope, Source, Target, RefreshSourceCache),
    UpdateCachedFlags(FolderName, Envelope, Target),
    UpdateFlags(FolderName, Envelope, Target),
    Uncache(FolderName, Id, Target),
    Delete(FolderName, Id, Target),
}

impl fmt::Display for EnvelopeSyncHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetThenCache(_folder, id, source) => {
                write!(f, "Adding envelope {id} to {source} cache")
            }
            Self::CopyThenCache(_folder, envelope, source, target, _) => {
                let id = &envelope.id;
                write!(f, "Copying {source} envelope {id} to {target} folder")
            }
            Self::UpdateCachedFlags(_folder, envelope, target) => {
                let id = &envelope.id;
                let flags = envelope.flags.to_string();
                write!(f, "Updating flags {flags} of {target} cached envelope {id}")
            }
            Self::UpdateFlags(_folder, envelope, target) => {
                let id = &envelope.id;
                let flags = envelope.flags.to_string();
                write!(f, "Setting flags {flags} of {target} envelope {id}")
            }
            Self::Uncache(_folder, id, target) => {
                write!(f, "Removing envelope {id} from {target} cache")
            }
            Self::Delete(_folder, id, target) => {
                write!(f, "Deleting {target} email {id}")
            }
        }
    }
}

impl EnvelopeSyncHunk {
    pub fn folder(&self) -> &str {
        match self {
            Self::GetThenCache(folder, _, _) => folder.as_str(),
            Self::CopyThenCache(folder, _, _, _, _) => folder.as_str(),
            Self::UpdateCachedFlags(folder, _, _) => folder.as_str(),
            Self::UpdateFlags(folder, _, _) => folder.as_str(),
            Self::Uncache(folder, _, _) => folder.as_str(),
            Self::Delete(folder, _, _) => folder.as_str(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EnvelopeSyncCacheHunk {
    Insert(FolderName, Envelope, Target),
    Delete(FolderName, Id, Target),
}
