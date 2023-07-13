//! Module dedicated to the Maildir backend.
//!
//! This module contains the implementation of the Maildir backend and
//! all associated structures related to it.

pub mod config;

use async_trait::async_trait;
use log::{debug, info, trace, warn};
use maildirpp::Maildir;
use std::{
    any::Any,
    env,
    ffi::OsStr,
    fs, io,
    path::{self, Path, PathBuf},
};
use thiserror::Error;

use crate::{
    account::{AccountConfig, DEFAULT_INBOX_FOLDER, DEFAULT_TRASH_FOLDER},
    backend::Backend,
    email::{Envelope, Envelopes, Flag, Flags, Messages},
    folder::{Folder, Folders},
    Result,
};

#[doc(inline)]
pub use self::config::MaildirConfig;

/// Errors related to the Maildir backend.
#[derive(Debug, Error)]
pub enum Error {
    // folders
    #[error("cannot init maildir folders structure at {1}")]
    InitFoldersStructureError(#[source] maildirpp::Error, PathBuf),
    #[error("cannot read maildir folder: invalid path {0}")]
    ReadFolderInvalidError(path::PathBuf),
    #[error("cannot parse maildir folder {0}")]
    ParseSubfolderError(path::PathBuf),
    #[error("cannot get maildir current folder")]
    GetCurrentFolderError(#[source] io::Error),
    #[error("cannot decode maildir subdirectory")]
    GetSubdirEntryError(#[source] maildirpp::Error),
    #[error("cannot delete maildir folder at {1}")]
    DeleteFolderError(#[source] io::Error, PathBuf),

    // envelopes
    #[error("cannot get envelope by short hash {0}")]
    GetEnvelopeError(String),
    #[error("cannot get maildir envelopes at page {0}")]
    GetEnvelopesOutOfBoundsError(usize),
    #[error("cannot search maildir envelopes: feature not implemented")]
    SearchEnvelopesUnimplementedError,

    // emails
    #[error("cannot store maildir message with flags")]
    StoreWithFlagsError(#[source] maildirpp::Error),
    #[error("cannot copy maildir message")]
    CopyEmailError(#[source] maildirpp::Error),
    #[error("cannot delete maildir message")]
    DeleteEmailError(#[source] maildirpp::Error),

    // flags
    #[error("cannot add maildir flags")]
    AddFlagsError(#[source] maildirpp::Error),
    #[error("cannot set maildir flags")]
    SetFlagsError(#[source] maildirpp::Error),
    #[error("cannot remove maildir flags")]
    RemoveFlagsError(#[source] maildirpp::Error),
}

/// The Maildir backend.
pub struct MaildirBackend {
    account_config: AccountConfig,
    mdir: Maildir,
}

impl MaildirBackend {
    /// Creates a new Maildir backend from configurations.
    pub fn new(account_config: AccountConfig, mdir_config: MaildirConfig) -> Result<Self> {
        let path = &mdir_config.root_dir;
        let mdir = Maildir::from(path.clone());

        mdir.create_dirs()
            .map_err(|err| Error::InitFoldersStructureError(err, path.clone()))?;

        Ok(Self {
            account_config,
            mdir,
        })
    }

    /// Returns a reference to the root Maildir directory path.
    pub fn path(&self) -> &Path {
        self.mdir.path()
    }

    /// Checks if the Maildir root directory is a valid path,
    /// otherwise returns an error.
    fn validate_mdir_path(&self, mdir_path: PathBuf) -> Result<PathBuf> {
        if mdir_path.is_dir() {
            Ok(mdir_path)
        } else {
            Ok(Err(Error::ReadFolderInvalidError(mdir_path.to_owned()))?)
        }
    }

    /// Creates a maildir instance from a path.
    pub fn get_mdir_from_dir(&self, folder: &str) -> Result<Maildir> {
        let folder = self.account_config.get_folder_alias(folder)?;

        // If the dir points to the inbox folder, creates a maildir
        // instance from the root folder.
        if folder == DEFAULT_INBOX_FOLDER {
            return self
                .validate_mdir_path(self.mdir.path().to_owned())
                .map(Maildir::from);
        }

        // If the dir is a valid maildir path, creates a maildir
        // instance from it. First checks for absolute path,
        self.validate_mdir_path((&folder).into())
            // then for relative path to `maildir-dir`,
            .or_else(|_| self.validate_mdir_path(self.mdir.path().join(&folder)))
            // and finally for relative path to the current directory.
            .or_else(|_| {
                self.validate_mdir_path(
                    env::current_dir()
                        .map_err(Error::GetCurrentFolderError)?
                        .join(&folder),
                )
            })
            .or_else(|_| {
                // Otherwise creates a maildir instance from a maildir
                // subdirectory by adding a "." in front of the name
                // as described in the [spec].
                //
                // [spec]: http://www.courier-mta.org/imap/README.maildirquota.html
                let folder = self.encode_folder(&folder);
                self.validate_mdir_path(self.mdir.path().join(format!(".{}", folder)))
            })
            .map(Maildir::from)
    }

    /// URL-encodes the given folder. The aim is to avoid naming
    /// issues due to special characters.
    pub fn encode_folder(&self, folder: impl AsRef<str> + ToString) -> String {
        urlencoding::encode(folder.as_ref()).to_string()
    }

    /// URL-decodes the given folder.
    pub fn decode_folder(&self, folder: impl AsRef<str> + ToString) -> String {
        urlencoding::decode(folder.as_ref())
            .map(|folder| folder.to_string())
            .unwrap_or_else(|_| folder.to_string())
    }
}

#[async_trait]
impl Backend for MaildirBackend {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    async fn add_folder(&mut self, folder: &str) -> Result<()> {
        info!("adding maildir folder {}", folder);

        let path = match self.account_config.get_folder_alias(folder)?.as_str() {
            DEFAULT_INBOX_FOLDER => self.mdir.path().join("cur"),
            folder => {
                let folder = self.encode_folder(folder);
                self.mdir.path().join(format!(".{}", folder))
            }
        };

        trace!("maildir folder path: {:?}", path);

        Maildir::from(path.clone())
            .create_dirs()
            .map_err(|err| Error::InitFoldersStructureError(err, path.clone()))?;

        Ok(())
    }

    async fn list_folders(&mut self) -> Result<Folders> {
        info!("listing maildir folders");

        let mut folders = Folders::default();

        folders.push(Folder {
            name: self.account_config.inbox_folder_alias()?,
            desc: DEFAULT_INBOX_FOLDER.into(),
        });

        for entry in self.mdir.list_subdirs() {
            let dir = entry.map_err(Error::GetSubdirEntryError)?;
            let dirname = dir.path().file_name();
            let name = dirname
                .and_then(OsStr::to_str)
                .and_then(|s| if s.len() < 2 { None } else { Some(&s[1..]) })
                .ok_or_else(|| Error::ParseSubfolderError(dir.path().to_owned()))?
                .to_string();

            if name == "notmuch" {
                continue;
            }

            folders.push(Folder {
                name: self.decode_folder(&name),
                desc: name,
            });
        }

        trace!("maildir folders: {:#?}", folders);

        Ok(folders)
    }

    async fn expunge_folder(&mut self, folder: &str) -> Result<()> {
        info!("expunging maildir folder {}", folder);

        let mdir = self.get_mdir_from_dir(folder)?;
        let entries = mdir
            .list_cur()
            .map(|entry| Ok(entry.map_err(Error::GetSubdirEntryError)?))
            .collect::<Result<Vec<_>>>()?;
        entries
            .iter()
            .filter_map(|entry| {
                if entry.is_trashed() {
                    Some(entry.id())
                } else {
                    None
                }
            })
            .try_for_each(|internal_id| {
                mdir.delete(internal_id).map_err(Error::DeleteEmailError)
            })?;

        Ok(())
    }

    async fn purge_folder(&mut self, folder: &str) -> Result<()> {
        info!("purging maildir folder {}", folder);

        let mdir = self.get_mdir_from_dir(folder)?;
        let entries = mdir
            .list_cur()
            .map(|entry| Ok(entry.map_err(Error::GetSubdirEntryError)?))
            .collect::<Result<Vec<_>>>()?;
        let ids = entries.iter().map(|entry| entry.id()).collect();

        trace!("ids: {:#?}", ids);

        self.delete_emails(folder, ids).await?;

        Ok(())
    }

    async fn delete_folder(&mut self, folder: &str) -> Result<()> {
        info!("deleting maildir folder {}", folder);

        let path = match self.account_config.get_folder_alias(folder)?.as_str() {
            DEFAULT_INBOX_FOLDER => self.mdir.path().join("cur"),
            folder => {
                let folder = self.encode_folder(folder);
                self.mdir.path().join(format!(".{}", folder))
            }
        };

        trace!("maildir folder path: {:?}", path);

        fs::remove_dir_all(&path).map_err(|err| Error::DeleteFolderError(err, path))?;

        Ok(())
    }

    async fn get_envelope(&mut self, folder: &str, internal_id: &str) -> Result<Envelope> {
        info!(
            "getting maildir envelope by internal id {} from folder {}",
            internal_id, folder
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let envelope: Envelope = Envelope::from_mdir_entry(
            mdir.find(internal_id)
                .ok_or_else(|| Error::GetEnvelopeError(internal_id.to_owned()))?,
        );

        Ok(envelope)
    }

    async fn list_envelopes(
        &mut self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        info!("listing maildir envelopes of folder {folder}");
        debug!("page size: {page_size}");
        debug!("page: {page}");

        let mdir = self.get_mdir_from_dir(folder)?;
        let mut envelopes = Envelopes::from_mdir_entries(mdir.list_cur());
        debug!("maildir envelopes: {envelopes:#?}");

        let page_begin = page * page_size;
        debug!("page begin: {}", page_begin);
        if page_begin > envelopes.len() {
            return Err(Error::GetEnvelopesOutOfBoundsError(page_begin + 1))?;
        }

        let page_end = envelopes.len().min(if page_size == 0 {
            envelopes.len()
        } else {
            page_begin + page_size
        });
        debug!("page end: {}", page_end);

        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        *envelopes = envelopes[page_begin..page_end].into();

        Ok(envelopes)
    }

    async fn search_envelopes(
        &mut self,
        _folder: &str,
        _query: &str,
        _sort: &str,
        _page_size: usize,
        _page: usize,
    ) -> Result<Envelopes> {
        Err(Error::SearchEnvelopesUnimplementedError)?
    }

    async fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String> {
        info!(
            "adding email to folder {folder} with flags {flags}",
            flags = flags.to_string()
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let internal_id = mdir
            .store_cur_with_flags(email, &flags.to_mdir_string())
            .map_err(Error::StoreWithFlagsError)?;

        Ok(internal_id)
    }

    async fn preview_emails(&mut self, folder: &str, internal_ids: Vec<&str>) -> Result<Messages> {
        info!(
            "previewing maildir emails by internal ids {ids} from folder {folder}",
            ids = internal_ids.join(", "),
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        let mut emails: Vec<(usize, maildirpp::MailEntry)> = mdir
            .list_cur()
            .filter_map(|entry| match entry {
                Ok(entry) => internal_ids
                    .iter()
                    .position(|id| *id == entry.id())
                    .map(|pos| (pos, entry)),
                Err(err) => {
                    warn!("skipping invalid maildir entry: {}", err);
                    None
                }
            })
            .collect();
        emails.sort_by_key(|(pos, _)| *pos);

        let emails: Messages = emails
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>()
            .try_into()?;

        Ok(emails)
    }

    async fn get_emails(&mut self, folder: &str, internal_ids: Vec<&str>) -> Result<Messages> {
        info!(
            "getting maildir emails by internal ids {ids} from folder {folder}",
            ids = internal_ids.join(", "),
        );

        let emails = self.preview_emails(folder, internal_ids.clone()).await?;
        self.add_flags(folder, internal_ids, &Flags::from_iter([Flag::Seen]))
            .await?;

        Ok(emails)
    }

    async fn copy_emails(
        &mut self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> Result<()> {
        info!(
            "copying internal ids {ids} from folder {from_folder} to folder {to_folder}",
            ids = internal_ids.join(", "),
        );

        let from_mdir = self.get_mdir_from_dir(from_folder)?;
        let to_mdir = self.get_mdir_from_dir(to_folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            from_mdir
                .copy_to(&internal_id, &to_mdir)
                .map_err(Error::CopyEmailError)
        })?;

        Ok(())
    }

    async fn move_emails(
        &mut self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> Result<()> {
        info!(
            "moving internal ids {ids} from folder {from_folder} to folder {to_folder}",
            ids = internal_ids.join(", "),
        );

        let from_mdir = self.get_mdir_from_dir(from_folder)?;
        let to_mdir = self.get_mdir_from_dir(to_folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            from_mdir
                .move_to(&internal_id, &to_mdir)
                .map_err(Error::CopyEmailError)
        })?;

        Ok(())
    }

    async fn delete_emails(&mut self, folder: &str, internal_ids: Vec<&str>) -> Result<()> {
        info!(
            "deleting internal ids {ids} from folder {folder}",
            ids = internal_ids.join(", "),
        );

        let trash_folder = self.account_config.trash_folder_alias()?;

        if self.account_config.get_folder_alias(folder)? == trash_folder {
            self.add_flags(folder, internal_ids, &Flags::from_iter([Flag::Deleted]))
                .await
        } else {
            self.move_emails(folder, DEFAULT_TRASH_FOLDER, internal_ids)
                .await
        }
    }

    async fn add_flags(
        &mut self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        info!(
            "adding flags {flags} to internal ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = internal_ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.add_flags(&internal_id, &flags.to_mdir_string())
                .map_err(Error::AddFlagsError)
        })?;

        Ok(())
    }

    async fn set_flags(
        &mut self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        info!(
            "setting flags {flags} to internal ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = internal_ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.set_flags(&internal_id, &flags.to_mdir_string())
                .map_err(Error::SetFlagsError)
        })?;

        Ok(())
    }

    async fn remove_flags(
        &mut self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        info!(
            "removing flags {flags} to internal ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = internal_ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.remove_flags(&internal_id, &flags.to_mdir_string())
                .map_err(Error::RemoveFlagsError)
        })?;

        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The Maildir backend builder.
///
/// Simple builder that helps to build a Maildir backend.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaildirBackendBuilder {
    account_config: AccountConfig,
    mdir_config: MaildirConfig,
}

impl MaildirBackendBuilder {
    /// Creates a new builder from configurations.
    pub fn new(account_config: AccountConfig, mdir_config: MaildirConfig) -> Self {
        Self {
            account_config,
            mdir_config,
        }
    }

    /// Builds the Maildir backend.
    pub fn build(&self) -> Result<MaildirBackend> {
        Ok(MaildirBackend::new(
            self.account_config.clone(),
            self.mdir_config.clone(),
        )?)
    }
}
