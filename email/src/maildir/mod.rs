mod config;

use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use shellexpand_utils::shellexpand_path;
use std::{
    env, io,
    ops::Deref,
    path::{self, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::{
    account::{AccountConfig, DEFAULT_INBOX_FOLDER},
    backend::BackendContextBuilder,
    Result,
};

#[doc(inline)]
pub use self::config::MaildirConfig;

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

        Ok(MaildirSessionSync {
            account_config: self.account_config,
            maildir_config: self.maildir_config,
            session: Arc::new(Mutex::new(session)),
        })
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

    /// Checks if the Maildir root directory is a valid path,
    /// otherwise returns an error.
    fn validate_mdir_path(&self, mdir_path: PathBuf) -> Result<PathBuf> {
        if mdir_path.is_dir() {
            Ok(mdir_path)
        } else {
            Err(Error::ReadFolderInvalidError(mdir_path.to_owned()).into())
        }
    }

    /// Creates a maildir instance from a path.
    pub fn get_mdir_from_dir(&self, folder: &str) -> Result<Maildir> {
        let folder = self.account_config.get_folder_alias(folder)?;

        // If the dir points to the inbox folder, creates a maildir
        // instance from the root folder.
        if folder == DEFAULT_INBOX_FOLDER {
            return self
                .validate_mdir_path(self.session.path().to_owned())
                .map(Maildir::from);
        }

        // If the dir is a valid maildir path, creates a maildir
        // instance from it. First checks for absolute path,
        self.validate_mdir_path((&folder).into())
            // then for relative path to `maildir-dir`,
            .or_else(|_| self.validate_mdir_path(self.session.path().join(&folder)))
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
                self.validate_mdir_path(self.session.path().join(format!(".{}", folder)))
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

/// The sync version of the Maildir session.
///
/// This is just a Maildir session wrapped into a mutex, so the same
/// Maildir session can be shared and updated across multiple threads.
#[derive(Clone)]
pub struct MaildirSessionSync {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The MAILDIR configuration.
    pub maildir_config: MaildirConfig,

    /// The MAILDIR session wrapped into a mutex.
    session: Arc<Mutex<MaildirSession>>,
}

impl MaildirSessionSync {
    /// Create a new MAILDIR sync session from an MAILDIR session.
    pub fn new(
        account_config: AccountConfig,
        maildir_config: MaildirConfig,
        session: MaildirSession,
    ) -> Self {
        Self {
            account_config,
            maildir_config,
            session: Arc::new(Mutex::new(session)),
        }
    }
}

impl Deref for MaildirSessionSync {
    type Target = Mutex<MaildirSession>;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}
