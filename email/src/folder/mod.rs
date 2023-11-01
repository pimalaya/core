//! Module dedicated to folder (alias mailbox) management.
//!
//! This module contains [`Folder`] and [`Folders`] representations.
//!
//! You also have everything you need to synchronize a remote folder
//! with a local one.

pub mod list;
pub mod sync;

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

/// The folder.
///
/// A folder is an email container. Depending on the [crate::Backend],
/// the folder is an alias for a mailbox (IMAP) or a directory
/// (Maildir/Notmuch).
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Folder {
    /// The folder name.
    pub name: String,

    /// The folder description.
    pub desc: String,
}

impl PartialEq for Folder {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl fmt::Display for Folder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// The list of folders.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
