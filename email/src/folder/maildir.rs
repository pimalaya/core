//! Module dedicated to Maildir email folders.
//!
//! This module contains folder-related mapping functions from the
//! [maildirpp] crate types.

use std::ffi::OsStr;

use maildirpp::{Maildir, Submaildirs};
use rayon::prelude::*;

use crate::{
    account::config::AccountConfig,
    debug,
    folder::{Folder, Folders},
    maildir, trace,
};

impl Folder {
    /// Parse a folder from a maildir instance.
    ///
    /// Returns [`None`] in case the folder name is too short (does
    /// not start by a dot) or is equal to `notmuch` (which should not
    /// be treated as a maildir folder).
    pub fn from_maildir(config: &AccountConfig, mdir: Maildir) -> Option<Self> {
        let folder = mdir
            .path()
            .file_name()
            .and_then(OsStr::to_str)
            .filter(|folder| folder.len() >= 2)
            .map(|folder| &folder[1..]);

        match folder {
            None => {
                debug!("cannot parse folder from maildir: invalid subdirectory name");
                None
            }
            Some("notmuch") => {
                debug!("skipping folder .notmuch");
                None
            }
            Some(name) => {
                let name = maildir::decode_folder(name);
                let kind = config
                    .find_folder_kind_from_alias(&name)
                    .or_else(|| name.parse().ok());
                let desc = mdir.path().to_owned().to_string_lossy().to_string();

                let folder = Folder { kind, name, desc };
                trace!("parsed maildir folder: {folder:#?}");
                Some(folder)
            }
        }
    }
}

impl Folders {
    /// Parse folders from submaildirs.
    ///
    /// Folders are parsed in parallel, using [`rayon`]. Only parses
    /// direct submaildirs (no recursion).
    pub fn from_submaildirs(config: &AccountConfig, submdirs: Submaildirs) -> Self {
        Folders::from_iter(
            submdirs
                .collect::<Vec<_>>()
                .into_par_iter()
                .filter_map(|mdir| match mdir {
                    Ok(mdir) => Folder::from_maildir(config, mdir),
                    Err(err) => {
                        debug!("cannot parse submaildir: {err}");
                        debug!("{err:?}");
                        None
                    }
                })
                .collect::<Vec<_>>(),
        )
    }
}
