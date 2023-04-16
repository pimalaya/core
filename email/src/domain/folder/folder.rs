//! Folder module.
//!
//! This module contains the representation of the email folder.

use std::fmt;

/// Represents the folder.
#[derive(Clone, Debug, Default, Eq, Hash)]
pub struct Folder {
    /// Represents the folder hierarchie delimiter.
    pub delim: String,
    /// Represents the folder name.
    pub name: String,
    /// Represents the folder description.
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
