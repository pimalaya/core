pub mod config;

use log::{error, info, trace, warn};
use maildirpp::Maildir;
use notmuch::{Database, DatabaseMode};
use std::{any::Any, fs, io, path::PathBuf, result};
use thiserror::Error;

use crate::{
    account, backend, email, AccountConfig, Backend, Envelope, Envelopes, Flag, Flags, Folder,
    Folders, Messages,
};

pub use self::config::NotmuchConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot validate maildir path {0}")]
    ValidatePathError(PathBuf),
    #[error("cannot canonicalize path {1}")]
    CanonicalizePath(#[source] io::Error, PathBuf),
    #[error("cannot get default notmuch database path")]
    GetDefaultDatabasePathError(#[source] notmuch::Error),
    #[error("cannot store notmuch email to folder {1}")]
    StoreWithFlagsError(#[source] maildirpp::Error, PathBuf),
    #[error("cannot find notmuch email")]
    FindMaildirEmailById,
    #[error("cannot open notmuch database at {1}")]
    OpenDatabaseError(#[source] rusqlite::Error, PathBuf),
    #[error("cannot find notmuch email")]
    FindEmailError(#[source] notmuch::Error),
    #[error("cannot remove tags from notmuch email {1}")]
    RemoveAllTagsError(#[source] notmuch::Error, String),

    #[error("cannot get notmuch backend from config")]
    GetBackendFromConfigError,
    #[error("cannot get notmuch inner maildir backend")]
    GetMaildirBackendError,
    #[error("cannot parse notmuch message header {1}")]
    GetHeaderError(#[source] notmuch::Error, String),
    #[error("cannot parse notmuch message date {1}")]
    ParseMsgDateError(#[source] chrono::ParseError, String),
    #[error("cannot find notmuch message header {0}")]
    FindMsgHeaderError(String),
    #[error("cannot find notmuch message sender")]
    FindSenderError,
    #[error("cannot open default notmuch database")]
    OpenDefaultNotmuchDatabaseError(#[source] notmuch::Error),
    #[error("cannot open notmuch database at {1}")]
    OpenNotmuchDatabaseError(#[source] notmuch::Error, PathBuf),
    #[error("cannot close notmuch database")]
    CloseDatabaseError(#[source] notmuch::Error),
    #[error("cannot build notmuch query")]
    BuildQueryError(#[source] notmuch::Error),
    #[error("cannot search notmuch envelopes")]
    SearchEnvelopesError(#[source] notmuch::Error),
    #[error("cannot get notmuch envelopes at page {0}")]
    GetEnvelopesOutOfBoundsError(usize),
    #[error("cannot add notmuch mailbox: feature not implemented")]
    AddMboxUnimplementedError,
    #[error("cannot purge notmuch folder: feature not implemented")]
    PurgeFolderUnimplementedError,
    #[error("cannot expunge notmuch folder: feature not implemented")]
    ExpungeFolderUnimplementedError,
    #[error("cannot delete notmuch mailbox: feature not implemented")]
    DeleteFolderUnimplementedError,
    #[error("cannot copy notmuch message: feature not implemented")]
    CopyMsgUnimplementedError,
    #[error("cannot move notmuch message: feature not implemented")]
    MoveMsgUnimplementedError,
    #[error("cannot index notmuch message")]
    IndexFileError(#[source] notmuch::Error),
    #[error("cannot find notmuch message")]
    FindMsgEmptyError,
    #[error("cannot read notmuch raw message from file")]
    ReadMsgError(#[source] io::Error),
    #[error("cannot delete notmuch message")]
    DelMsgError(#[source] notmuch::Error),
    #[error("cannot add notmuch tag")]
    AddTagError(#[source] notmuch::Error),
    #[error("cannot delete notmuch tag")]
    RemoveTagError(#[source] notmuch::Error),

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    MaildirError(#[from] backend::maildir::Error),
}

type Result<T> = result::Result<T, Error>;

/// Represents the Notmuch backend.
pub struct NotmuchBackend {
    account_config: AccountConfig,
    notmuch_config: NotmuchConfig,
    db: Database,
}

impl NotmuchBackend {
    pub fn new(account_config: AccountConfig, notmuch_config: NotmuchConfig) -> Result<Self> {
        let path = Self::path_from(&notmuch_config);
        let db = Database::open_with_config(
            Some(&path),
            DatabaseMode::ReadWrite,
            None as Option<PathBuf>,
            None,
        )
        .map_err(|err| Error::OpenNotmuchDatabaseError(err, path.clone()))?;

        Ok(Self {
            account_config,
            notmuch_config,
            db,
        })
    }

    pub fn get_default_db_path() -> Result<PathBuf> {
        Ok(Database::open_with_config(
            None as Option<PathBuf>,
            DatabaseMode::ReadWrite,
            None as Option<PathBuf>,
            None,
        )
        .map_err(Error::OpenDefaultNotmuchDatabaseError)?
        .path()
        .to_owned())
    }

    fn path_from(notmuch_config: &NotmuchConfig) -> PathBuf {
        notmuch_config
            .db_path
            .to_str()
            .and_then(|path| shellexpand::full(path).ok())
            .map(|path| PathBuf::from(path.to_string()))
            .and_then(|path| path.canonicalize().ok())
            .unwrap_or_else(|| notmuch_config.db_path.clone())
    }

    pub fn path(&self) -> PathBuf {
        Self::path_from(&self.notmuch_config)
    }

    fn _search_envelopes(&self, query: &str, page_size: usize, page: usize) -> Result<Envelopes> {
        let query_builder = self
            .db
            .create_query(query)
            .map_err(Error::BuildQueryError)?;

        let mut envelopes = Envelopes::from(
            query_builder
                .search_messages()
                .map_err(Error::SearchEnvelopesError)?,
        );
        trace!("notmuch envelopes: {envelopes:#?}");

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
        *envelopes = envelopes[page_begin..page_end].into();

        Ok(envelopes)
    }
}

impl Backend for NotmuchBackend {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    fn add_folder(&mut self, _folder: &str) -> backend::Result<()> {
        Err(Error::AddMboxUnimplementedError)?
    }

    fn list_folders(&mut self) -> backend::Result<Folders> {
        let mut mboxes = Folders::default();
        for (name, desc) in &self.account_config.folder_aliases {
            mboxes.push(Folder {
                name: name.into(),
                desc: desc.into(),
                ..Folder::default()
            })
        }
        mboxes.sort_by(|a, b| b.name.partial_cmp(&a.name).unwrap());

        trace!("notmuch virtual folders: {:?}", mboxes);
        Ok(mboxes)
    }

    fn expunge_folder(&mut self, _folder: &str) -> backend::Result<()> {
        Err(Error::PurgeFolderUnimplementedError)?
    }

    fn purge_folder(&mut self, _folder: &str) -> backend::Result<()> {
        Err(Error::ExpungeFolderUnimplementedError)?
    }

    fn delete_folder(&mut self, _folder: &str) -> backend::Result<()> {
        Err(Error::DeleteFolderUnimplementedError)?
    }

    fn get_envelope(&mut self, _folder: &str, internal_id: &str) -> backend::Result<Envelope> {
        info!("getting notmuch envelope by internal id {internal_id}");

        let envelope = Envelope::try_from(
            self.db
                .find_message(&internal_id)
                .map_err(Error::FindEmailError)?
                .ok_or_else(|| Error::FindMsgEmptyError)?,
        )?;
        trace!("envelope: {envelope:#?}");

        Ok(envelope)
    }

    fn list_envelopes(
        &mut self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("listing notmuch envelopes from virtual folder {folder}");

        let query = self
            .account_config
            .folder_alias(folder.as_ref())
            .map(|folder| format!("folder:{folder:?}"))?;
        trace!("query: {query}");
        let envelopes = self._search_envelopes(&query, page_size, page)?;
        trace!("envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        _sort: &str,
        page_size: usize,
        page: usize,
    ) -> backend::Result<Envelopes> {
        info!("searching notmuch envelopes from folder {folder}");

        let folder_query = self
            .account_config
            .folder_alias(folder.as_ref())
            .map(|folder| format!("folder:{folder:?}"))?;
        let query = if query.is_empty() {
            folder_query
        } else {
            folder_query + " and " + query.as_ref()
        };
        trace!("query: {query}");

        let envelopes = self._search_envelopes(&query, page_size, page)?;
        trace!("envelopes: {envelopes:#?}");

        Ok(envelopes)
    }

    fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> backend::Result<String> {
        info!(
            "adding notmuch email with flags {flags}",
            flags = flags.to_string()
        );

        let folder = self.account_config.folder_alias(folder)?;
        let path = self.path().join(folder);
        let mdir = Maildir::from(
            path.canonicalize()
                .map_err(|err| Error::CanonicalizePath(err, path.clone()))?,
        );
        let mdir_internal_id = mdir
            .store_cur_with_flags(email, &flags.to_normalized_string())
            .map_err(|err| Error::StoreWithFlagsError(err, mdir.path().to_owned()))?;
        trace!("added email internal maildir id: {mdir_internal_id}");

        let entry = mdir
            .find(&mdir_internal_id)
            .ok_or(Error::FindMaildirEmailById)?;
        let path = entry
            .path()
            .canonicalize()
            .map_err(|err| Error::CanonicalizePath(err, entry.path().clone()))?;
        trace!("path: {path:?}");

        let email = self
            .db
            .index_file(&path, None)
            .map_err(Error::IndexFileError)?;

        Ok(email.id().to_string())
    }

    fn preview_emails(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
    ) -> backend::Result<Messages> {
        info!(
            "previewing notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        let msgs: Messages = internal_ids
            .iter()
            .map(|internal_id| {
                let email_filepath = self
                    .db
                    .find_message(&internal_id)
                    .map_err(Error::FindEmailError)?
                    .ok_or_else(|| Error::FindMsgEmptyError)?
                    .filename()
                    .to_owned();
                fs::read(&email_filepath).map_err(Error::ReadMsgError)
            })
            .collect::<Result<Vec<_>>>()?
            .into();

        Ok(msgs)
    }

    fn get_emails(&mut self, folder: &str, internal_ids: Vec<&str>) -> backend::Result<Messages> {
        info!(
            "getting notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );
        let emails = self.preview_emails(folder, internal_ids.clone())?;
        self.add_flags("INBOX", internal_ids, &Flags::from_iter([Flag::Seen]))?;
        Ok(emails)
    }

    fn copy_emails(
        &mut self,
        _from_dir: &str,
        _to_dir: &str,
        _internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
        // How to deal with duplicate Message-ID?
        Err(Error::CopyMsgUnimplementedError)?
    }

    fn move_emails(
        &mut self,
        _from_dir: &str,
        _to_dir: &str,
        _internal_ids: Vec<&str>,
    ) -> backend::Result<()> {
        Err(Error::MoveMsgUnimplementedError)?
    }

    fn delete_emails(&mut self, _folder: &str, internal_ids: Vec<&str>) -> backend::Result<()> {
        info!(
            "deleting notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        internal_ids.iter().try_for_each(|internal_id| {
            let path = self
                .db
                .find_message(&internal_id)
                .map_err(Error::FindEmailError)?
                .ok_or_else(|| Error::FindMsgEmptyError)?
                .filename()
                .to_owned();
            self.db.remove_message(path).map_err(Error::DelMsgError)
        })?;

        Ok(())
    }

    fn add_flags(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "adding notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        let query_builder = self
            .db
            .create_query(&query)
            .map_err(Error::BuildQueryError)?;
        let emails = query_builder
            .search_messages()
            .map_err(Error::SearchEnvelopesError)?;

        for email in emails {
            for flag in flags.iter() {
                email
                    .add_tag(&flag.to_string())
                    .map_err(Error::AddTagError)?;
            }
        }

        Ok(())
    }

    fn set_flags(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "setting notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        let query_builder = self
            .db
            .create_query(&query)
            .map_err(Error::BuildQueryError)?;
        let emails = query_builder
            .search_messages()
            .map_err(Error::SearchEnvelopesError)?;

        for email in emails {
            email
                .remove_all_tags()
                .map_err(|err| Error::RemoveAllTagsError(err, email.id().to_string()))?;

            for flag in flags.iter() {
                email
                    .add_tag(&flag.to_string())
                    .map_err(Error::AddTagError)?;
            }
        }

        Ok(())
    }

    fn remove_flags(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> backend::Result<()> {
        info!(
            "removing notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        let query_builder = self
            .db
            .create_query(&query)
            .map_err(Error::BuildQueryError)?;
        let emails = query_builder
            .search_messages()
            .map_err(Error::SearchEnvelopesError)?;

        for email in emails {
            for flag in flags.iter() {
                email
                    .remove_tag(&flag.to_string())
                    .map_err(Error::RemoveTagError)?;
            }
        }

        Ok(())
    }

    fn close(&mut self) -> backend::Result<()> {
        Ok(self.db.close().map_err(Error::CloseDatabaseError)?)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for NotmuchBackend {
    fn drop(&mut self) {
        if let Err(err) = self.close() {
            warn!("cannot close notmuch database, skipping it");
            error!("cannot close notmuch database: {err:?}");
        }
    }
}

pub struct NotmuchBackendBuilder {
    account_config: AccountConfig,
    mdir_config: NotmuchConfig,
}

impl NotmuchBackendBuilder {
    pub fn new(account_config: AccountConfig, mdir_config: NotmuchConfig) -> Self {
        Self {
            account_config,
            mdir_config,
        }
    }

    pub fn build(&self) -> Result<NotmuchBackend> {
        Ok(NotmuchBackend::new(
            self.account_config.clone(),
            self.mdir_config.clone(),
        )?)
    }
}
