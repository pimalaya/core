use std::fmt;

use crate::{
    backend::sync::{Id, RefreshSourceCache, Source, Target},
    folder::sync::FolderName,
    Envelope,
};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum EnvelopeSyncHunk {
    GetEnvelopeThenCacheIt(FolderName, Id, Source),
    CopyEmailThenCacheIt(FolderName, Envelope, Source, Target, RefreshSourceCache),
    UpdateCachedFlags(FolderName, Envelope, Target),
    SetFlags(FolderName, Envelope, Target),
    DeleteCachedEnvelope(FolderName, Id, Target),
    RemoveEmail(FolderName, Id, Target),
}

impl fmt::Display for EnvelopeSyncHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetEnvelopeThenCacheIt(_folder, id, source) => {
                write!(f, "Adding envelope {id} to {source} cache")
            }
            Self::CopyEmailThenCacheIt(_folder, envelope, source, target, _) => {
                let id = &envelope.id;
                write!(f, "Copying {source} envelope {id} to {target} folder")
            }
            Self::UpdateCachedFlags(_folder, envelope, target) => {
                let id = &envelope.id;
                let flags = envelope.flags.to_string();
                write!(f, "Updating flags {flags} of {target} cached envelope {id}")
            }
            Self::SetFlags(_folder, envelope, target) => {
                let id = &envelope.id;
                let flags = envelope.flags.to_string();
                write!(f, "Setting flags {flags} of {target} envelope {id}")
            }
            Self::DeleteCachedEnvelope(_folder, id, target) => {
                write!(f, "Removing envelope {id} from {target} cache")
            }
            Self::RemoveEmail(_folder, id, target) => {
                write!(f, "Removing {target} email {id}")
            }
        }
    }
}

impl EnvelopeSyncHunk {
    pub fn folder(&self) -> &str {
        match self {
            Self::GetEnvelopeThenCacheIt(folder, _, _) => folder.as_str(),
            Self::CopyEmailThenCacheIt(folder, _, _, _, _) => folder.as_str(),
            Self::UpdateCachedFlags(folder, _, _) => folder.as_str(),
            Self::SetFlags(folder, _, _) => folder.as_str(),
            Self::DeleteCachedEnvelope(folder, _, _) => folder.as_str(),
            Self::RemoveEmail(folder, _, _) => folder.as_str(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EnvelopeSyncCacheHunk {
    InsertEnvelope(FolderName, Envelope, Target),
    DeleteEnvelope(FolderName, Id, Target),
}
