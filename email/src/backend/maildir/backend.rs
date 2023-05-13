//! Maildir backend module.
//!
//! This module contains the definition of the maildir backend and its
//! traits implementation.

use log::{info, trace, warn};
use maildir::Maildir;
use pimalaya_id_alias::IdAlias;
use std::{
    any::Any,
    borrow::Cow,
    env,
    ffi::OsStr,
    fs, io,
    path::{self, PathBuf},
    result,
};
use thiserror::Error;

use crate::{
    account::{self, config::DEFAULT_TRASH_FOLDER},
    backend, email, AccountConfig, Backend, Emails, Envelope, Envelopes, Flag, Flags, Folder,
    Folders, MaildirConfig, DEFAULT_INBOX_FOLDER,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open maildir email file at {1}")]
    OpenEmailFileError(#[source] io::Error, PathBuf),
    #[error("cannot read maildir email line at {1}")]
    ReadEmailLineError(#[source] io::Error, PathBuf),

    #[error("cannot open maildir database at {1}")]
    OpenDatabaseError(#[source] rusqlite::Error, PathBuf),
    #[error("cannot init maildir folders structure at {1}")]
    InitFoldersStructureError(#[source] io::Error, PathBuf),
    #[error("cannot delete folder at {1}")]
    DeleteFolderError(#[source] io::Error, PathBuf),
    #[error(transparent)]
    IdAliasError(#[from] pimalaya_id_alias::Error),

    #[error("cannot parse timestamp from maildir envelope: {1}")]
    ParseTimestampFromMaildirEnvelopeError(mailparse::MailParseError, String),

    #[error("cannot parse header date as timestamp")]
    ParseDateHeaderError,
    #[error("cannot get envelope by short hash {0}")]
    GetEnvelopeError(String),
    #[error("cannot get maildir backend from config")]
    GetBackendFromConfigError,
    #[error("cannot find maildir sender")]
    FindSenderError,
    #[error("cannot read maildir directory {0}")]
    ReadDirError(path::PathBuf),
    #[error("cannot parse maildir subdirectory {0}")]
    ParseSubdirError(path::PathBuf),
    #[error("cannot get maildir envelopes at page {0}")]
    GetEnvelopesOutOfBoundsError(usize),
    #[error("cannot search maildir envelopes: feature not implemented")]
    SearchEnvelopesUnimplementedError,
    #[error("cannot get maildir message {0}")]
    GetMsgError(String),
    #[error("cannot decode maildir entry")]
    DecodeEntryError(#[source] io::Error),
    #[error("cannot parse maildir message")]
    ParseMsgError(#[source] maildir::MailEntryError),
    #[error("cannot decode header {0}")]
    DecodeHeaderError(#[source] rfc2047_decoder::Error, String),
    #[error("cannot parse maildir message header {0}")]
    ParseHeaderError(#[source] mailparse::MailParseError, String),
    #[error("cannot create maildir subdirectory {1}")]
    CreateSubdirError(#[source] io::Error, String),
    #[error("cannot decode maildir subdirectory")]
    GetSubdirEntryError(#[source] io::Error),
    #[error("cannot get current directory")]
    GetCurrentDirError(#[source] io::Error),
    #[error("cannot store maildir message with flags")]
    StoreWithFlagsError(#[source] maildir::MaildirError),
    #[error("cannot copy maildir message")]
    CopyEmailError(#[source] io::Error),
    #[error("cannot move maildir message")]
    MoveMsgError(#[source] io::Error),
    #[error("cannot delete maildir message")]
    DeleteEmailError(#[source] io::Error),
    #[error("cannot add maildir flags")]
    AddFlagsError(#[source] io::Error),
    #[error("cannot set maildir flags")]
    SetFlagsError(#[source] io::Error),
    #[error("cannot remove maildir flags")]
    RemoveFlagsError(#[source] io::Error),

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    EmailError(#[from] email::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the maildir backend.
pub struct MaildirBackend<'a> {
    account_config: Cow<'a, AccountConfig>,
    mdir: maildir::Maildir,
    db_path: PathBuf,
}

const ID_MAPPER_DB_FILE_NAME: &str = ".id-mapper.sqlite";

impl<'a> MaildirBackend<'a> {
    pub fn new(
        account_config: Cow<'a, AccountConfig>,
        backend_config: Cow<'a, MaildirConfig>,
    ) -> Result<Self> {
        let path = &backend_config.root_dir;
        let mdir = Maildir::from(path.clone());

        let mut db_path = mdir.path().join(ID_MAPPER_DB_FILE_NAME);
        let mut db_parent_dir = mdir.path().parent();

        while !db_path.is_file() {
            match db_parent_dir {
                Some(dir) => {
                    db_path = dir.join(ID_MAPPER_DB_FILE_NAME);
                    db_parent_dir = dir.parent();
                }
                None => {
                    db_path = mdir.path().join(ID_MAPPER_DB_FILE_NAME);
                    break;
                }
            }
        }

        mdir.create_dirs()
            .map_err(|err| Error::InitFoldersStructureError(err, path.clone()))?;

        let maildir_backend = Self {
            account_config,
            mdir,
            db_path,
        };

        // spawns a fake id mapper to init the database
        maildir_backend.id_mapper(DEFAULT_INBOX_FOLDER)?;

        Ok(maildir_backend)
    }

    fn validate_mdir_path(&self, mdir_path: PathBuf) -> Result<PathBuf> {
        if mdir_path.is_dir() {
            Ok(mdir_path)
        } else {
            Err(Error::ReadDirError(mdir_path.to_owned()))
        }
    }

    /// Creates a maildir instance from a string slice.
    pub fn get_mdir_from_dir(&self, folder: &str) -> Result<Maildir> {
        let folder = self.account_config.folder_alias(folder)?;
        let folder = self.encode_folder(&folder).to_string();

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
                        .map_err(Error::GetCurrentDirError)?
                        .join(&folder),
                )
            })
            .or_else(|_| {
                // Otherwise creates a maildir instance from a maildir
                // subdirectory by adding a "." in front of the name
                // as described in the [spec].
                //
                // [spec]: http://www.courier-mta.org/imap/README.maildirquota.html
                self.validate_mdir_path(self.mdir.path().join(format!(".{}", folder)))
            })
            .map(Maildir::from)
    }

    pub fn get_email_path<F, I>(&self, folder: F, id: I) -> Result<PathBuf>
    where
        F: AsRef<str> + ToString,
        I: AsRef<str> + ToString,
    {
        let internal_id = self
            .id_mapper(folder.as_ref())?
            .get_id(id.as_ref().parse::<i64>().unwrap())?;
        self.get_email_path_internal(internal_id)
    }

    pub fn get_email_path_internal<I>(&self, internal_id: I) -> Result<PathBuf>
    where
        I: AsRef<str> + ToString,
    {
        Ok(self
            .mdir
            .find(internal_id.as_ref())
            .ok_or_else(|| Error::GetEnvelopeError(internal_id.to_string()))?
            .path()
            .to_owned())
    }

    pub fn encode_folder<F>(&self, folder: F) -> String
    where
        F: AsRef<str> + ToString,
    {
        urlencoding::encode(folder.as_ref()).to_string()
    }

    pub fn decode_folder<F>(&self, folder: F) -> String
    where
        F: AsRef<str> + ToString,
    {
        urlencoding::decode(folder.as_ref())
            .map(|folder| folder.to_string())
            .unwrap_or_else(|_| folder.to_string())
    }

    pub fn id_mapper<F>(&self, folder: F) -> Result<IdAlias>
    where
        F: AsRef<str>,
    {
        let key = self.account_config.name.clone() + folder.as_ref();
        Ok(IdAlias::new(&self.db_path, key)?)
    }
}

impl<'a> Backend for MaildirBackend<'a> {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    fn add_folder(&self, folder: &str) -> backend::Result<()> {
        info!("adding maildir folder {}", folder);

        let path = match self.account_config.folder_alias(folder)?.as_str() {
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

    fn list_folders(&self) -> backend::Result<Folders> {
        info!("listing maildir folders");

        let mut folders = Folders::default();

        folders.push(Folder {
            delim: String::from("/"),
            name: self.account_config.inbox_folder_alias()?,
            desc: DEFAULT_INBOX_FOLDER.into(),
        });

        for entry in self.mdir.list_subdirs() {
            let dir = entry.map_err(Error::GetSubdirEntryError)?;
            let dirname = dir.path().file_name();
            let name = dirname
                .and_then(OsStr::to_str)
                .and_then(|s| if s.len() < 2 { None } else { Some(&s[1..]) })
                .ok_or_else(|| Error::ParseSubdirError(dir.path().to_owned()))?
                .to_string();

            folders.push(Folder {
                delim: String::from("/"),
                name: self.decode_folder(&name),
                desc: name,
            });
        }

        trace!("maildir folders: {:#?}", folders);

        Ok(folders)
    }

    fn expunge_folder(&self, folder: &str) -> backend::Result<()> {
        info!("expunging maildir folder {}", folder);

        let mdir = self.get_mdir_from_dir(folder)?;
        let entries = mdir
            .list_cur()
            .map(|entry| entry.map_err(Error::GetSubdirEntryError))
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

    fn purge_folder(&self, folder: &str) -> backend::Result<()> {
        info!("purging maildir folder {}", folder);

        let mdir = self.get_mdir_from_dir(folder)?;
        let entries = mdir
            .list_cur()
            .map(|entry| entry.map_err(Error::GetSubdirEntryError))
            .collect::<Result<Vec<_>>>()?;
        let ids = entries.iter().map(|entry| entry.id()).collect();

        trace!("ids: {:#?}", ids);

        self.delete_emails(folder, ids)?;

        Ok(())
    }

    fn delete_folder(&self, folder: &str) -> backend::Result<()> {
        info!("deleting maildir folder {}", folder);

        let path = match self.account_config.folder_alias(folder)?.as_str() {
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

    fn get_envelope(&self, folder: &str, id: &str) -> backend::Result<Envelope> {
        info!(
            "getting maildir envelope by id {} from folder {}",
            id, folder
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let internal_id = self.id_mapper(folder)?.get_id(id.parse::<i64>().unwrap())?;
        let mut envelope = Envelope::try_from(
            mdir.find(&internal_id)
                .ok_or_else(|| Error::GetEnvelopeError(id.to_owned()))?,
        )?;
        envelope.id = id.to_string();

        Ok(envelope)
    }

    fn get_envelope_internal(&self, folder: &str, internal_id: &str) -> backend::Result<Envelope> {
        info!(
            "getting maildir envelope by internal id {} from folder {}",
            internal_id, folder
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let mut envelope = Envelope::try_from(
            mdir.find(internal_id)
                .ok_or_else(|| Error::GetEnvelopeError(internal_id.to_owned()))?,
        )?;
        envelope.id = self
            .id_mapper(folder)?
            .get_or_create_alias(internal_id)?
            .to_string();

        Ok(envelope)
    }

    fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("listing maildir envelopes of folder {}", folder);
        trace!("page size: {}", page_size);
        trace!("page: {}", page);

        let mdir = self.get_mdir_from_dir(folder)?;
        let id_mapper = self.id_mapper(folder)?;
        let mut envelopes = Envelopes::try_from(mdir.list_cur())?;

        let page_begin = page * page_size;
        trace!("page begin: {}", page_begin);
        if page_begin > envelopes.len() {
            return Err(Error::GetEnvelopesOutOfBoundsError(page_begin + 1))?;
        }

        let page_end = envelopes.len().min(if page_size == 0 {
            envelopes.len()
        } else {
            page_begin + page_size
        });
        trace!("page end: {}", page_end);

        envelopes.sort_by(|a, b| b.date.partial_cmp(&a.date).unwrap());
        *envelopes = envelopes[page_begin..page_end]
            .iter()
            .map(|envelope| {
                Ok(Envelope {
                    id: id_mapper
                        .get_or_create_alias(&envelope.internal_id)?
                        .to_string(),
                    ..envelope.clone()
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(envelopes)
    }

    fn search_envelopes(
        &self,
        _folder: &str,
        _query: &str,
        _sort: &str,
        _page_size: usize,
        _page: usize,
    ) -> backend::Result<Envelopes> {
        Err(Error::SearchEnvelopesUnimplementedError)?
    }

    fn add_email(&self, folder: &str, email: &[u8], flags: &Flags) -> backend::Result<String> {
        info!(
            "adding email to folder {folder} with flags {flags}",
            flags = flags.to_string()
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let internal_id = mdir
            .store_cur_with_flags(email, &flags.to_normalized_string())
            .map_err(Error::StoreWithFlagsError)?;
        let id = self.id_mapper(folder)?.create(internal_id)?.to_string();

        Ok(id)
    }

    fn add_email_internal(
        &self,
        folder: &str,
        email: &[u8],
        flags: &Flags,
    ) -> backend::Result<String> {
        info!(
            "adding email to folder {folder} with flags {flags}",
            flags = flags.to_string()
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let internal_id = mdir
            .store_cur_with_flags(email, &flags.to_normalized_string())
            .map_err(Error::StoreWithFlagsError)?;
        self.id_mapper(folder)?.create(&internal_id)?;

        Ok(internal_id)
    }

    fn preview_emails(&self, folder: &str, ids: Vec<&str>) -> backend::Result<Emails> {
        info!(
            "previewing maildir emails by ids {ids} from folder {folder}",
            ids = ids.join(", "),
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let id_mapper = self.id_mapper(folder)?;
        let internal_ids: Vec<String> = ids
            .iter()
            .map(|id| Ok(id_mapper.get_id(id.parse::<i64>().unwrap())?))
            .collect::<Result<_>>()?;
        let internal_ids: Vec<&str> = internal_ids.iter().map(String::as_str).collect();
        trace!("internal ids: {:#?}", internal_ids);

        let mut emails: Vec<(usize, maildir::MailEntry)> = mdir
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

        let emails: Emails = emails
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>()
            .try_into()?;

        Ok(emails)
    }

    fn preview_emails_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<Emails> {
        info!(
            "previewing maildir emails by internal ids {ids} from folder {folder}",
            ids = internal_ids.join(", "),
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        let mut emails: Vec<(usize, maildir::MailEntry)> = mdir
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

        let emails: Emails = emails
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>()
            .try_into()?;

        Ok(emails)
    }

    fn get_emails(&self, folder: &str, ids: Vec<&str>) -> backend::Result<Emails> {
        info!(
            "getting maildir emails by ids {ids} from folder {folder}",
            ids = ids.join(", "),
        );

        let emails = self.preview_emails(folder, ids.clone())?;
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Seen]))?;

        Ok(emails)
    }

    fn get_emails_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<Emails> {
        info!(
            "getting maildir emails by internal ids {ids} from folder {folder}",
            ids = internal_ids.join(", "),
        );

        let emails = self.preview_emails_internal(folder, internal_ids.clone())?;
        self.add_flags_internal(folder, internal_ids, &Flags::from_iter([Flag::Seen]))?;

        Ok(emails)
    }

    fn copy_emails(
        &self,
        from_folder: &str,
        to_folder: &str,
        ids: Vec<&str>,
    ) -> backend::Result<()> {
        info!(
            "copying ids {ids} from folder {from_folder} to folder {to_folder}",
            ids = ids.join(", "),
        );

        let from_mdir = self.get_mdir_from_dir(from_folder)?;
        let to_mdir = self.get_mdir_from_dir(to_folder)?;
        let id_mapper = self.id_mapper(from_folder)?;
        let internal_ids: Vec<String> = ids
            .iter()
            .map(|id| Ok(id_mapper.get_id(id.parse::<i64>().unwrap())?))
            .collect::<Result<_>>()?;
        let internal_ids: Vec<&str> = internal_ids.iter().map(String::as_str).collect();
        trace!("internal ids: {:#?}", internal_ids);

        internal_ids.iter().try_for_each(|internal_id| {
            from_mdir
                .copy_to(&internal_id, &to_mdir)
                .map_err(Error::CopyEmailError)
        })?;

        Ok(())
    }

    fn copy_emails_internal(
        &self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
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

    fn move_emails(
        &self,
        from_folder: &str,
        to_folder: &str,
        ids: Vec<&str>,
    ) -> backend::Result<()> {
        info!(
            "moving ids {ids} from folder {from_folder} to folder {to_folder}",
            ids = ids.join(", "),
        );

        let from_mdir = self.get_mdir_from_dir(from_folder)?;
        let to_mdir = self.get_mdir_from_dir(to_folder)?;
        let id_mapper = self.id_mapper(from_folder)?;
        let internal_ids: Vec<String> = ids
            .iter()
            .map(|id| Ok(id_mapper.get_id(id.parse::<i64>().unwrap())?))
            .collect::<Result<_>>()?;
        let internal_ids: Vec<&str> = internal_ids.iter().map(String::as_str).collect();
        trace!("internal ids: {:#?}", internal_ids);

        internal_ids.iter().try_for_each(|internal_id| {
            from_mdir
                .move_to(&internal_id, &to_mdir)
                .map_err(Error::CopyEmailError)
        })?;

        Ok(())
    }

    fn move_emails_internal(
        &self,
        from_folder: &str,
        to_folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
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

    fn delete_emails(&self, folder: &str, ids: Vec<&str>) -> backend::Result<()> {
        info!(
            "deleting ids {ids} from folder {folder}",
            ids = ids.join(", "),
        );

        let trash_folder = self.account_config.trash_folder_alias()?;

        if self.account_config.folder_alias(folder)? == trash_folder {
            self.mark_emails_as_deleted(folder, ids)
        } else {
            self.move_emails(&folder, DEFAULT_TRASH_FOLDER, ids)
        }
    }

    fn delete_emails_internal(&self, folder: &str, internal_ids: Vec<&str>) -> backend::Result<()> {
        info!(
            "deleting internal ids {ids} from folder {folder}",
            ids = internal_ids.join(", "),
        );

        let trash_folder = self.account_config.trash_folder_alias()?;

        if self.account_config.folder_alias(folder)? == trash_folder {
            self.add_flags_internal(folder, internal_ids, &Flags::from_iter([Flag::Deleted]))
        } else {
            self.move_emails_internal(folder, DEFAULT_TRASH_FOLDER, internal_ids)
        }
    }

    fn add_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        info!(
            "adding flags {flags} to ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let id_mapper = self.id_mapper(folder)?;
        let internal_ids: Vec<String> = ids
            .iter()
            .map(|id| Ok(id_mapper.get_id(id.parse::<i64>().unwrap())?))
            .collect::<Result<_>>()?;
        let internal_ids: Vec<&str> = internal_ids.iter().map(String::as_str).collect();
        trace!("internal ids: {:#?}", internal_ids);

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.add_flags(&internal_id, &flags.to_normalized_string())
                .map_err(Error::AddFlagsError)
        })?;

        Ok(())
    }

    fn add_flags_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "adding flags {flags} to internal ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = internal_ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.add_flags(&internal_id, &flags.to_normalized_string())
                .map_err(Error::AddFlagsError)
        })?;

        Ok(())
    }

    fn set_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        info!(
            "setting flags {flags} to ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let id_mapper = self.id_mapper(folder)?;
        let internal_ids: Vec<String> = ids
            .iter()
            .map(|id| Ok(id_mapper.get_id(id.parse::<i64>().unwrap())?))
            .collect::<Result<_>>()?;
        let internal_ids: Vec<&str> = internal_ids.iter().map(String::as_str).collect();
        trace!("internal ids: {:#?}", internal_ids);

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.set_flags(&internal_id, &flags.to_normalized_string())
                .map_err(Error::SetFlagsError)
        })?;

        Ok(())
    }

    fn set_flags_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "setting flags {flags} to internal ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = internal_ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.set_flags(&internal_id, &flags.to_normalized_string())
                .map_err(Error::SetFlagsError)
        })?;

        Ok(())
    }

    fn remove_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> backend::Result<()> {
        info!(
            "removing flags {flags} to ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;
        let id_mapper = self.id_mapper(folder)?;
        let internal_ids: Vec<String> = ids
            .iter()
            .map(|id| Ok(id_mapper.get_id(id.parse::<i64>().unwrap())?))
            .collect::<Result<_>>()?;
        let internal_ids: Vec<&str> = internal_ids.iter().map(String::as_str).collect();
        trace!("internal ids: {:#?}", internal_ids);

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.remove_flags(&internal_id, &flags.to_normalized_string())
                .map_err(Error::RemoveFlagsError)
        })?;

        Ok(())
    }

    fn remove_flags_internal(
        &self,
        folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "removing flags {flags} to internal ids {ids} from folder {folder}",
            flags = flags.to_string(),
            ids = internal_ids.join(", ")
        );

        let mdir = self.get_mdir_from_dir(folder)?;

        internal_ids.iter().try_for_each(|internal_id| {
            mdir.remove_flags(&internal_id, &flags.to_normalized_string())
                .map_err(Error::RemoveFlagsError)
        })?;

        Ok(())
    }

    fn as_any(&'static self) -> &(dyn Any) {
        self
    }
}
