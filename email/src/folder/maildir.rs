//! Module dedicated to Maildir email folders.
//!
//! This module contains folder-related mapping functions from the
//! [maildirpp] crate types.

use maildirs::Maildir;

use crate::{
    account::config::AccountConfig,
    folder::{Folder, Folders},
    maildir::MaildirContext,
};

use super::Result;

impl Folders {
    /// Parse folders from submaildirs.
    ///
    /// Folders are parsed in parallel, using [`rayon`]. Only parses
    /// direct submaildirs (no recursion).
    pub fn from_maildir_context(ctx: &MaildirContext) -> Self {
        Folders::from_iter(ctx.root.iter().map(|entry| {
            Folder {
                kind: ctx
                    .account_config
                    .find_folder_kind_from_alias(&entry.name)
                    .or_else(|| entry.name.parse().ok()),
                name: entry.name,
                desc: entry.maildir.path().display().to_string(),
            }
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
