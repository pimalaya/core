//! Module dedicated to the Notmuch backend.
//!
//! This module contains the implementation of the Notmuch backend and
//! all associated structures related to it.

pub mod config;

use async_trait::async_trait;
use log::{error, info, trace};
use maildirpp::Maildir;
use notmuch::{Database, DatabaseMode};
use std::{any::Any, fs, io, path::PathBuf};
use thiserror::Error;

use crate::{
    account::AccountConfig,
    backend::Backend,
    email::{Envelope, Envelopes, Flag, Flags, Messages},
    folder::{Folder, Folders},
    Result,
};

#[doc(inline)]
pub use self::config::NotmuchConfig;

/// Errors related to the Notmuch backend.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot canonicalize path {1}")]
    CanonicalizePathError(#[source] io::Error, PathBuf),
    #[error("cannot store notmuch email to folder {1}")]
    StoreWithFlagsError(#[source] maildirpp::Error, PathBuf),
    #[error("cannot find notmuch email")]
    FindMaildirEmailById,
    #[error("cannot find notmuch email")]
    FindEmailError(#[source] notmuch::Error),
    #[error("cannot remove tags from notmuch email {1}")]
    RemoveAllTagsError(#[source] notmuch::Error, String),

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
}

/// The Notmuch backend.
pub struct NotmuchBackend {
    account_config: AccountConfig,
    notmuch_config: NotmuchConfig,
}

impl NotmuchBackend {
    /// Creates a new Notmuch backend from configurations.
    pub fn new(account_config: AccountConfig, notmuch_config: NotmuchConfig) -> Result<Self> {
        Ok(Self {
            account_config,
            notmuch_config,
        })
    }

    /// Returns the default Notmuch database path from the notmuch
    /// lib.
    ///
    /// The default path comes from the Notmuch user configuration
    /// file `~/.notmuchrc`.
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

    /// Returns the Notmuch database path.
    ///
    /// Tries first the path from the Himalaya configuration file,
    /// falls back to the default Notmuch database path from the
    /// notmuch lib.
    fn path_from(notmuch_config: &NotmuchConfig) -> PathBuf {
        notmuch_config
            .db_path
            .to_str()
            .and_then(|path| shellexpand::full(path).ok())
            .map(|path| PathBuf::from(path.to_string()))
            .and_then(|path| path.canonicalize().ok())
            .unwrap_or_else(|| notmuch_config.db_path.clone())
    }

    /// Returns the Notmuch database path.
    pub fn path(&self) -> PathBuf {
        Self::path_from(&self.notmuch_config)
    }

    /// Opens a notmuch database then passes a reference of it to the
    /// given callback function.
    fn open_db(&self) -> Result<Database> {
        let path = Self::path_from(&self.notmuch_config);
        let db = Database::open_with_config(
            Some(&path),
            DatabaseMode::ReadWrite,
            None as Option<PathBuf>,
            None,
        )
        .map_err(|err| Error::OpenNotmuchDatabaseError(err, path.clone()))?;
        Ok(db)
    }

    /// Closes the given notmuch database.
    fn close_db(db: Database) -> Result<()> {
        Ok(db.close().map_err(Error::CloseDatabaseError)?)
    }

    /// Searches envelopes matching the given Notmuch query and the
    /// given pagination.
    fn _search_envelopes(&self, query: &str, page_size: usize, page: usize) -> Result<Envelopes> {
        let db = self.open_db()?;

        let query_builder = db.create_query(query).map_err(Error::BuildQueryError)?;

        let mut envelopes = Envelopes::from_notmuch_msgs(
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

        Self::close_db(db)?;
        Ok(envelopes)
    }
}

#[async_trait]
impl Backend for NotmuchBackend {
    fn name(&self) -> String {
        self.account_config.name.clone()
    }

    async fn add_folder(&mut self, _folder: &str) -> Result<()> {
        Err(Error::AddMboxUnimplementedError)?
    }

    async fn list_folders(&mut self) -> Result<Folders> {
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

    async fn expunge_folder(&mut self, _folder: &str) -> Result<()> {
        Err(Error::PurgeFolderUnimplementedError)?
    }

    async fn purge_folder(&mut self, _folder: &str) -> Result<()> {
        Err(Error::ExpungeFolderUnimplementedError)?
    }

    async fn delete_folder(&mut self, _folder: &str) -> Result<()> {
        Err(Error::DeleteFolderUnimplementedError)?
    }

    async fn get_envelope(&mut self, _folder: &str, internal_id: &str) -> Result<Envelope> {
        info!("getting notmuch envelope by internal id {internal_id}");

        let db = self.open_db()?;

        let envelope: Envelope = Envelope::from_notmuch_msg(
            db.find_message(&internal_id)
                .map_err(Error::FindEmailError)?
                .ok_or_else(|| Error::FindMsgEmptyError)?,
        );
        trace!("notmuch envelope: {envelope:#?}");

        Self::close_db(db)?;
        Ok(envelope)
    }

    async fn list_envelopes(
        &mut self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
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

    async fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        _sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
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

    async fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String> {
        info!(
            "adding notmuch email with flags {flags}",
            flags = flags.to_string()
        );

        let db = self.open_db()?;

        let folder = self.account_config.folder_alias(folder)?;
        let path = self.path().join(folder);
        let mdir = Maildir::from(
            path.canonicalize()
                .map_err(|err| Error::CanonicalizePathError(err, path.clone()))?,
        );
        let mdir_internal_id = mdir
            .store_cur_with_flags(email, &flags.to_mdir_string())
            .map_err(|err| Error::StoreWithFlagsError(err, mdir.path().to_owned()))?;
        trace!("added email internal maildir id: {mdir_internal_id}");

        let entry = mdir
            .find(&mdir_internal_id)
            .ok_or(Error::FindMaildirEmailById)?;
        let path = entry
            .path()
            .canonicalize()
            .map_err(|err| Error::CanonicalizePathError(err, entry.path().clone()))?;
        trace!("path: {path:?}");

        let email = db.index_file(&path, None).map_err(Error::IndexFileError)?;

        Self::close_db(db)?;
        Ok(email.id().to_string())
    }

    async fn preview_emails(&mut self, _folder: &str, internal_ids: Vec<&str>) -> Result<Messages> {
        info!(
            "previewing notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        let db = self.open_db()?;

        let msgs: Messages = internal_ids
            .iter()
            .map(|internal_id| {
                let email_filepath = db
                    .find_message(&internal_id)
                    .map_err(Error::FindEmailError)?
                    .ok_or_else(|| Error::FindMsgEmptyError)?
                    .filename()
                    .to_owned();
                let email = fs::read(&email_filepath).map_err(Error::ReadMsgError)?;
                Ok(email)
            })
            .collect::<Result<Vec<_>>>()?
            .into();

        Self::close_db(db)?;
        Ok(msgs)
    }

    async fn get_emails(&mut self, folder: &str, internal_ids: Vec<&str>) -> Result<Messages> {
        info!(
            "getting notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        let emails = self.preview_emails(folder, internal_ids.clone()).await?;
        self.add_flags("INBOX", internal_ids, &Flags::from_iter([Flag::Seen]))
            .await?;

        Ok(emails)
    }

    async fn copy_emails(
        &mut self,
        _from_dir: &str,
        _to_dir: &str,
        _internal_ids: Vec<&str>,
    ) -> Result<()> {
        // How to deal with duplicate Message-ID?
        Err(Error::CopyMsgUnimplementedError)?
    }

    async fn move_emails(
        &mut self,
        _from_dir: &str,
        _to_dir: &str,
        _internal_ids: Vec<&str>,
    ) -> Result<()> {
        Err(Error::MoveMsgUnimplementedError)?
    }

    async fn delete_emails(&mut self, _folder: &str, internal_ids: Vec<&str>) -> Result<()> {
        info!(
            "deleting notmuch emails by internal ids {ids}",
            ids = internal_ids.join(", ")
        );

        let db = self.open_db()?;

        internal_ids.iter().try_for_each(|internal_id| {
            let path = db
                .find_message(&internal_id)
                .map_err(Error::FindEmailError)?
                .ok_or_else(|| Error::FindMsgEmptyError)?
                .filename()
                .to_owned();
            db.remove_message(path).map_err(Error::DelMsgError)
        })?;

        Self::close_db(db)?;
        Ok(())
    }

    async fn add_flags(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        info!(
            "adding notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let db = self.open_db()?;

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
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

        Self::close_db(db)?;
        Ok(())
    }

    async fn set_flags(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        info!(
            "setting notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let db = self.open_db()?;

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
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

        Self::close_db(db)?;
        Ok(())
    }

    async fn remove_flags(
        &mut self,
        _folder: &str,
        internal_ids: Vec<&str>,
        flags: &Flags,
    ) -> Result<()> {
        info!(
            "removing notmuch flags {flags} by internal_ids {ids}",
            flags = flags.to_string(),
            ids = internal_ids.join(", "),
        );

        let db = self.open_db()?;

        let query = format!("mid:\"/^({})$/\"", internal_ids.join("|"));
        trace!("query: {query}");

        let query_builder = db.create_query(&query).map_err(Error::BuildQueryError)?;
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

        Self::close_db(db)?;
        Ok(())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The Notmuch backend builder.
///
/// Simple builder that helps to build a Notmuch backend.
pub struct NotmuchBackendBuilder {
    account_config: AccountConfig,
    notmuch_config: NotmuchConfig,
}

impl NotmuchBackendBuilder {
    /// Creates a new builder from configurations.
    pub fn new(account_config: AccountConfig, notmuch_config: NotmuchConfig) -> Self {
        Self {
            account_config,
            notmuch_config,
        }
    }

    /// Builds the Notmuch backend.
    pub fn build(&self) -> Result<NotmuchBackend> {
        Ok(NotmuchBackend::new(
            self.account_config.clone(),
            self.notmuch_config.clone(),
        )?)
    }
}
