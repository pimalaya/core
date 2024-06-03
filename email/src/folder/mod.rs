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
mod error;
pub mod expunge;
#[cfg(feature = "imap")]
pub mod imap;
pub mod list;
#[cfg(feature = "maildir")]
pub mod maildir;
pub mod purge;
#[cfg(feature = "sync")]
pub mod sync;

use std::{
    fmt,
    hash::Hash,
    ops::{Deref, DerefMut},
    str::FromStr,
};

#[cfg(feature = "sync")]
pub(crate) use sync::sync;

#[doc(inline)]
pub use self::error::{Error, Result};

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

    /// The user-defined kind of folder.
    ///
    /// This kind of folder represents the alias as defined by the
    /// user in [`config::FolderConfig`]::aliases.
    UserDefined(String),
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

    /// Return `true` if the current folder kind matches the
    /// UserDefined variant.
    pub fn is_user_defined(&self) -> bool {
        matches!(self, FolderKind::UserDefined(_))
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

    /// Return the folder kind as string slice.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Inbox => INBOX,
            Self::Sent => SENT,
            Self::Drafts => DRAFTS,
            Self::Trash => TRASH,
            Self::UserDefined(alias) => alias.as_str(),
        }
    }
}

impl FromStr for FolderKind {
    type Err = Error;

    fn from_str(kind: &str) -> Result<Self> {
        match kind {
            kind if kind.eq_ignore_ascii_case(INBOX) => Ok(Self::Inbox),
            kind if kind.eq_ignore_ascii_case(SENT) => Ok(Self::Sent),
            kind if kind.eq_ignore_ascii_case(DRAFT) => Ok(Self::Drafts),
            kind if kind.eq_ignore_ascii_case(DRAFTS) => Ok(Self::Drafts),
            kind if kind.eq_ignore_ascii_case(TRASH) => Ok(Self::Trash),
            kind => Err(Error::ParseFolderKindError(kind.to_owned())),
        }
    }
}

impl<T: AsRef<str>> From<T> for FolderKind {
    fn from(kind: T) -> Self {
        kind.as_ref()
            .parse()
            .ok()
            .unwrap_or_else(|| Self::UserDefined(kind.as_ref().to_owned()))
    }
}

impl fmt::Display for FolderKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// The folder structure.
///
/// The folder is just a container for emails. Depending on the
/// backend used, the folder can be seen as a mailbox (IMAP/JMAP) or
/// as a system directory (Maildir).
#[derive(Clone, Debug, Default, Eq)]
pub struct Folder {
    /// The optional folder kind.
    pub kind: Option<FolderKind>,

    /// The folder name.
    pub name: String,

    /// The folder description.
    ///
    /// The description depends on the backend used: it can be IMAP
    /// attributes or Maildir path.
    pub desc: String,
}

impl Folder {
    /// Return `true` if the folder kind matches the Inbox variant.
    pub fn is_inbox(&self) -> bool {
        self.kind
            .as_ref()
            .map(|kind| kind.is_inbox())
            .unwrap_or_default()
    }

    /// Return `true` if the folder kind matches the Sent variant.
    pub fn is_sent(&self) -> bool {
        self.kind
            .as_ref()
            .map(|kind| kind.is_sent())
            .unwrap_or_default()
    }

    /// Return `true` if the folder kind matches the Drafts variant.
    pub fn is_drafts(&self) -> bool {
        self.kind
            .as_ref()
            .map(|kind| kind.is_drafts())
            .unwrap_or_default()
    }

    /// Return `true` if the folder kind matches the Trash variant.
    pub fn is_trash(&self) -> bool {
        self.kind
            .as_ref()
            .map(|kind| kind.is_trash())
            .unwrap_or_default()
    }

    /// Return the folder kind as string slice if existing, otherwise
    /// return the folder name as string slice.
    pub fn get_kind_or_name(&self) -> &str {
        self.kind
            .as_ref()
            .map(FolderKind::as_str)
            .unwrap_or(self.name.as_str())
    }
}

impl PartialEq for Folder {
    fn eq(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (Some(self_kind), Some(other_kind)) => self_kind == other_kind,
            (None, None) => self.name == other.name,
            _ => false,
        }
        // self.kind == other.kind || self.name == other.name
    }
}
impl Hash for Folder {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.kind {
            Some(kind) => kind.hash(state),
            None => self.name.hash(state),
        }
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

impl IntoIterator for Folders {
    type Item = Folder;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Folder> for Folders {
    fn from_iter<T: IntoIterator<Item = Folder>>(iter: T) -> Self {
        let mut folders = Folders::default();
        folders.extend(iter);
        folders
    }
}

impl From<Folders> for Vec<Folder> {
    fn from(val: Folders) -> Self {
        val.0
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::hash_map::DefaultHasher, hash::Hasher};

    use super::*;
    fn folder_inbox_foo() -> Folder {
        Folder {
            kind: Some(FolderKind::Inbox),
            name: "foo".to_owned(),
            desc: "1".to_owned(),
        }
    }
    fn folder_none_foo() -> Folder {
        Folder {
            kind: None,
            name: "foo".to_owned(),
            desc: "2".to_owned(),
        }
    }
    fn folder_none_bar() -> Folder {
        Folder {
            kind: None,
            name: "bar".to_owned(),
            desc: "3".to_owned(),
        }
    }
    fn folder_inbox_bar() -> Folder {
        Folder {
            kind: Some(FolderKind::Inbox),
            name: "bar".to_owned(),
            desc: "4".to_owned(),
        }
    }

    fn hash<H: Hash>(item: H) -> u64 {
        let mut hasher = DefaultHasher::new();
        item.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn folder_inbox_bar_equals_inbox_foo_test() {
        assert_eq!(folder_inbox_bar(), folder_inbox_foo());
    }

    #[test]
    fn folder_inbox_bar_equals_inbox_foo_test_hash() {
        assert_eq!(hash(folder_inbox_bar()), hash(folder_inbox_foo()));
    }

    #[test]
    fn folder_none_foo_not_equals_inbox_foo_test() {
        assert_ne!(folder_none_foo(), folder_inbox_foo());
    }

    #[test]
    fn folder_none_foo_not_equals_inbox_foo_test_hash() {
        assert_ne!(hash(folder_none_foo()), hash(folder_inbox_foo()));
    }

    #[test]
    fn folder_none_foo_not_equals_none_bar_test() {
        assert_ne!(folder_none_foo(), folder_none_bar());
    }

    #[test]
    fn folder_none_foo_not_equals_none_bar_test_hash() {
        assert_ne!(hash(folder_none_foo()), hash(folder_none_bar()));
    }
}
