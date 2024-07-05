//! Module dedicated to Maildir email folders.
//!
//! This module contains folder-related mapping functions from the
//! [maildirpp] crate types.

use maildirs::Maildir;

use crate::{
    account::config::AccountConfig,
    debug,
    folder::{Folder, Folders},
};

use super::Result;

impl Folders {
    /// Parse folders from submaildirs.
    ///
    /// Folders are parsed in parallel, using [`rayon`]. Only parses
    /// direct submaildirs (no recursion).
    pub fn from_maildirs(config: &AccountConfig, maildirs: impl Iterator<Item = Maildir>) -> Self {
        Folders::from_iter(maildirs.filter_map(|mdir| {
            let folder = Folder::try_from_maildir(config, mdir);

            // #[cfg(feature = "tracing")]
            if let Err(err) = folder {
                debug!(err, "cannot parse folder from maildir, skipping it");
            }

            folder.ok()
        }))
    }
}

impl Folder {
    /// Parse a folder from a maildir instance.
    ///
    /// Returns [`None`] in case the folder name is too short (does
    /// not start by a dot) or is equal to `notmuch` (which should not
    /// be treated as a maildir folder).
    pub fn try_from_maildir(config: &AccountConfig, mdir: Maildir) -> Result<Self> {
        let name = mdir.name()?.to_owned();
        let kind = config
            .find_folder_kind_from_alias(&name)
            .or_else(|| name.parse().ok());
        let desc = mdir.path().display().to_string();

        Ok(Folder { kind, name, desc })
    }
}
