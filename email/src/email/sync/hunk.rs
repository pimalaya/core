//! Module dedicated to emails synchronization hunk.
//!
//! The core structure of the module is the [`EmailSyncHunk`], which
//! represents a change in a patch.

use std::fmt;

use crate::{
    account::sync::{Source, Target},
    envelope::Envelope,
    folder::sync::FolderName,
};

/// Alias for the email identifier (Message-ID).
pub type Id = String;

/// Flag for refreshing source cache.
pub type RefreshSourceCache = bool;

/// The email synchronization hunk.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum EmailSyncHunk {
    /// The email matching the given identifier from the given folder
    /// needs to be retrieved for the given source then cached.
    GetThenCache(FolderName, Id, Source),

    /// The email matching the given envelope id from the given folder
    /// needs to be copied from the given source to the given target
    /// then cached if the refresh flag is `true`.
    CopyThenCache(FolderName, Envelope, Source, Target, RefreshSourceCache),

    /// The envelope matching the given envelope identifier from the
    /// given folder needs to refresh its flags cache for the given
    /// target.
    UpdateCachedFlags(FolderName, Envelope, Target),

    /// The envelope matching the given envelope identifier from the
    /// given folder needs to update its flags for the given target.
    UpdateFlags(FolderName, Envelope, Target),

    /// The envelope matching the given identifier from the given
    /// folder needs to be removed from the cache for the given
    /// target.
    Uncache(FolderName, Id, Target),

    /// The envelope matching the given identifier from the given
    /// folder needs to be deleted from the given target.
    Delete(FolderName, Id, Target),
}

impl fmt::Display for EmailSyncHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetThenCache(folder, id, source) => {
                write!(f, "Adding envelope {id} to {source} cache ({folder})")
            }
            Self::CopyThenCache(folder, envelope, source, target, _) => {
                let id = &envelope.id;
                write!(
                    f,
                    "Copying {source} envelope {id} to {target} folder {folder}"
                )
            }
            Self::UpdateCachedFlags(folder, envelope, target) => {
                let id = &envelope.id;
                let flags = envelope.flags.to_string();
                write!(
                    f,
                    "Updating flags {flags} of {target} cached envelope {id} ({folder})"
                )
            }
            Self::UpdateFlags(folder, envelope, target) => {
                let id = &envelope.id;
                let flags = envelope.flags.to_string();
                write!(
                    f,
                    "Setting flags {flags} of {target} envelope {id} ({folder})"
                )
            }
            Self::Uncache(folder, id, target) => {
                write!(f, "Removing envelope {id} from {target} cache ({folder})")
            }
            Self::Delete(folder, id, target) => {
                write!(f, "Deleting {target} email {id} ({folder})")
            }
        }
    }
}

impl EmailSyncHunk {
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

/// The email synchronization cache hunk.
///
/// Similar to the [`EmailSyncHunk`], except that this hunk is
/// specific to the cache (SQLite).
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EmailSyncCacheHunk {
    /// The email matching the given envelope identifier needs to be
    /// added to the cache for the given destination.
    Insert(FolderName, Envelope, Target),

    /// The email matching the given identifier needs to be removed
    /// from the cache for the given destination.
    Delete(FolderName, Id, Target),
}
