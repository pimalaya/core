//! # Folder module
//!
//! Module dedicated to folder (as known as mailbox) management.
//!
//! The main entities are [`FolderKind`], [`Folder`] and [`Folders`].
//!
//! The [`config`] module exposes all the folder configuration used by
//! the account configuration.
//!
//! Backend features reside in their own module as well: [`add`],
//! [`list`], [`expunge`], [`purge`], [`delete`].
//!
//! Finally, the [`sync`] module contains everything needed to
//! synchronize a remote folder with a local one.

pub mod add;
pub mod config;
pub mod delete;
pub mod expunge;
#[cfg(feature = "imap")]
pub mod imap;
pub mod list;
#[cfg(feature = "maildir")]
pub mod maildir;
pub mod purge;
pub mod sync;

use std::{
    fmt,
    ops::{Deref, DerefMut},
    str::FromStr,
};
use thiserror::Error;

/// Errors dedicated to folder management.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse folder kind {0}")]
    ParseFolderKindError(String),
}

pub const INBOX: &str = "INBOX";
pub const SENT: &str = "Sent";
pub const DRAFT: &str = "Drafts";
pub const DRAFTS: &str = "Drafts";
pub const TRASH: &str = "Trash";

/// The folder kind enumeration.
///
/// The folder kind is a category that gives a specific purpose to a
/// folder. It is used internally by the library to operate on the
/// right folder.
///
/// [`FolderConfig::aliases`](crate::folder::config::FolderConfig)
/// allows users to map custom folder names but also to map the
/// following folder kinds.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum FolderKind {
    /// The kind of folder that contains received emails.
    ///
    /// This folder kind is mostly used for listing new or recent
    /// emails.
    Inbox,

    /// The kind of folder that contains sent emails.
    ///
    /// This folder kind is used to store a copy of sent emails.
    Sent,

    /// The kind of folder than contains not finished emails.
    ///
    /// This kind of folder is used to store drafts. Emails in this
    /// folder are supposed to be edited. Once completed they should
    /// be removed from the folder.
    Drafts,

    /// The kind of folder that contains trashed emails.
    ///
    /// This kind of folder is used as a trash bin. Emails contained
    /// in this folder are supposed to be deleted.
    Trash,
}

impl FolderKind {
    /// Return `true` if the current folder kind matches the Inbox
    /// variant.
    pub fn is_inbox(&self) -> bool {
        matches!(self, FolderKind::Inbox)
    }

    /// Return `true` if the current folder kind matches the Sent
    /// variant.
    pub fn is_sent(&self) -> bool {
        matches!(self, FolderKind::Sent)
    }

    /// Return `true` if the current folder kind matches the Drafts
    /// variant.
    pub fn is_drafts(&self) -> bool {
        matches!(self, FolderKind::Drafts)
    }

    /// Return `true` if the current folder kind matches the Trash
    /// variant.
    pub fn is_trash(&self) -> bool {
        matches!(self, FolderKind::Trash)
    }

    /// Return `true` if the give string matches the Inbox variant.
    pub fn matches_inbox(folder: impl AsRef<str>) -> bool {
        folder
            .as_ref()
            .parse::<FolderKind>()
            .map(|kind| kind.is_inbox())
            .unwrap_or_default()
    }

    /// Return `true` if the given string matches the Sent variant.
    pub fn matches_sent(folder: impl AsRef<str>) -> bool {
        folder
            .as_ref()
            .parse::<FolderKind>()
            .map(|kind| kind.is_sent())
            .unwrap_or_default()
    }

    /// Return `true` if the given string matches the Drafts variant.
    pub fn matches_drafts(folder: impl AsRef<str>) -> bool {
        folder
            .as_ref()
            .parse::<FolderKind>()
            .map(|kind| kind.is_drafts())
            .unwrap_or_default()
    }

    /// Return `true` if the given string matches the Trash variant.
    pub fn matches_trash(folder: impl AsRef<str>) -> bool {
        folder
            .as_ref()
            .parse::<FolderKind>()
            .map(|kind| kind.is_trash())
            .unwrap_or_default()
    }
}

impl FromStr for FolderKind {
    type Err = Error;

    fn from_str(kind: &str) -> Result<Self, Self::Err> {
        match kind {
            kind if kind.eq_ignore_ascii_case(INBOX) => Ok(Self::Inbox),
            kind if kind.eq_ignore_ascii_case(SENT) => Ok(Self::Sent),
            kind if kind.eq_ignore_ascii_case(DRAFT) => Ok(Self::Drafts),
            kind if kind.eq_ignore_ascii_case(DRAFTS) => Ok(Self::Drafts),
            kind if kind.eq_ignore_ascii_case(TRASH) => Ok(Self::Trash),
            kind => Err(Error::ParseFolderKindError(kind.to_owned()).into()),
        }
    }
}

impl fmt::Display for FolderKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Inbox => write!(f, "{INBOX}"),
            Self::Sent => write!(f, "{SENT}"),
            Self::Drafts => write!(f, "{DRAFTS}"),
            Self::Trash => write!(f, "{TRASH}"),
        }
    }
}

/// The folder structure.
///
/// The folder is just a container for emails. Depending on the
/// backend used, the folder can be seen as a mailbox (IMAP/JMAP) or
/// as a system directory (Maildir).
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Folder {
    /// The optional folder kind.
    pub kind: Option<FolderKind>,

    /// The folder name.
    pub name: String,

    /// The folder description.
    pub desc: String,
}

impl PartialEq for Folder {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind || self.name == other.name
    }
}

impl fmt::Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.kind {
            Some(kind) => write!(f, "{kind}"),
            None => write!(f, "{}", self.name),
        }
    }
}

/// The list of folders.
///
/// This structure is just a convenient wrapper used to implement
/// custom mappers for backends.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Folders(Vec<Folder>);

impl Deref for Folders {
    type Target = Vec<Folder>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Folders {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<Folder> for Folders {
    fn from_iter<T: IntoIterator<Item = Folder>>(iter: T) -> Self {
        let mut folders = Folders::default();
        folders.extend(iter);
        folders
    }
}

impl Into<Vec<Folder>> for Folders {
    fn into(self) -> Vec<Folder> {
        self.0
    }
}
