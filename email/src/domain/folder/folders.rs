//! Folders module.
//!
//! This module contains the representation of the email folders.

use std::ops::{Deref, DerefMut};

use crate::Folder;

/// Represents the list of folders.
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
