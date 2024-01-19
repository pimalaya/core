pub mod config;

use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use shellexpand_utils::{shellexpand_path, try_shellexpand_path};
use std::{
    io,
    ops::Deref,
    path::{self, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    account::config::AccountConfig, backend::BackendContextBuilder, folder::FolderKind, maildir,
    Result,
};

use self::config::MaildirConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[error("maildir: cannot init folders structure at {1}")]
    InitFoldersStructureError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot read folder: invalid path {0}")]
    ReadFolderInvalidError(path::PathBuf),
    #[error("maildir: cannot get current folder")]
    GetCurrentFolderError(#[source] io::Error),
}

/// The Maildir session builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MaildirSessionBuilder {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The Maildir configuration.
    pub maildir_config: MaildirConfig,
}

impl MaildirSessionBuilder {
    pub fn new(account_config: AccountConfig, maildir_config: MaildirConfig) -> Self {
        Self {
            account_config,
            maildir_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for MaildirSessionBuilder {
    type Context = MaildirSessionSync;

    /// Build an IMAP sync session.
    ///
    /// The IMAP session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self) -> Result<Self::Context> {
        info!("building new maildir session");

        let path = shellexpand_path(&self.maildir_config.root_dir);

        let session = MaildirSession {
            account_config: self.account_config.clone(),
            maildir_config: self.maildir_config.clone(),
            session: Maildir::from(path),
        };

        session.create_dirs()?;

        Ok(Arc::new(Mutex::new(session)))
    }
}

/// The Maildir session.
///
/// This session is unsync, which means it cannot be shared between
/// threads. For the sync version, see [`MaildirSessionSync`].
pub struct MaildirSession {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The Maildir configuration.
    pub maildir_config: MaildirConfig,

    /// The current Maildir session.
    session: Maildir,
}

impl Deref for MaildirSession {
    type Target = Maildir;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl MaildirSession {
    pub fn create_dirs(&self) -> Result<()> {
        self.session
            .create_dirs()
            .map_err(|err| Error::InitFoldersStructureError(err, self.session.path().to_owned()))?;
        Ok(())
    }

    /// Creates a maildir instance from a folder name.
    pub fn get_maildir_from_folder_name(&self, folder: &str) -> Result<Maildir> {
        // If the folder matches to the inbox folder kind, create a
        // maildir instance from the root folder.
        if FolderKind::matches_inbox(folder) {
            return try_shellexpand_path(self.session.path().to_owned())
                .map(Maildir::from)
                .map_err(Into::into);
        }

        let folder = self.account_config.get_folder_alias(folder);

        // If the folder is a valid maildir path, creates a maildir
        // instance from it. First check for absolute path…
        try_shellexpand_path(&folder)
            // then check for relative path to `maildir-dir`…
            .or_else(|_| try_shellexpand_path(self.session.path().join(&folder)))
            // TODO: should move to CLI
            // // and finally check for relative path to the current
            // // directory
            // .or_else(|_| {
            //     try_shellexpand_path(
            //         env::current_dir()
            //             .map_err(Error::GetCurrentFolderError)?
            //             .join(&folder),
            //     )
            // })
            .or_else(|_| {
                // Otherwise creates a maildir instance from a maildir
                // subdirectory by adding a "." in front of the name
                // as described in the [spec].
                //
                // [spec]: http://www.courier-mta.org/imap/README.maildirquota.html
                let folder = maildir::encode_folder(&folder);
                try_shellexpand_path(self.session.path().join(format!(".{}", folder)))
            })
            .map(Maildir::from)
            .map_err(Into::into)
    }
}

/// The sync version of the Maildir session.
///
/// This is just a Maildir session wrapped into a mutex, so the same
/// Maildir session can be shared and updated across multiple threads.
pub type MaildirSessionSync = Arc<Mutex<MaildirSession>>;

/// URL-encodes the given folder. The aim is to avoid naming
/// issues due to special characters.
pub fn encode_folder(folder: impl AsRef<str> + ToString) -> String {
    urlencoding::encode(folder.as_ref()).to_string()
}

/// URL-decodes the given folder.
pub fn decode_folder(folder: impl AsRef<str> + ToString) -> String {
    urlencoding::decode(folder.as_ref())
        .map(|folder| folder.to_string())
        .unwrap_or_else(|_| folder.to_string())
}
