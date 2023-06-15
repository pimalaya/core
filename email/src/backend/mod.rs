mod config;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;
pub mod sync;

use log::error;
use std::{any::Any, result};
use thiserror::Error;

pub use self::config::BackendConfig;
#[cfg(feature = "imap-backend")]
pub use self::imap::*;
pub use self::maildir::*;
#[cfg(feature = "notmuch-backend")]
pub use self::notmuch::*;
pub use self::sync::{
    BackendSyncBuilder, BackendSyncProgress, BackendSyncProgressEvent, BackendSyncReport,
};
use crate::{account, message, AccountConfig, Envelope, Envelopes, Flag, Flags, Folders, Messages};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build backend with an empty config")]
    BuildBackendError,

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    MessageError(#[from] message::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapBackendError(#[from] imap::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapBackendConfigError(#[from] imap::config::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] maildir::Error),
    #[cfg(feature = "notmuch-backend")]
    #[error(transparent)]
    NotmuchBackendError(#[from] notmuch::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub trait Backend {
    fn name(&self) -> String;

    fn add_folder(&mut self, folder: &str) -> Result<()>;
    fn list_folders(&mut self) -> Result<Folders>;
    fn expunge_folder(&mut self, folder: &str) -> Result<()>;
    fn purge_folder(&mut self, folder: &str) -> Result<()>;
    fn delete_folder(&mut self, folder: &str) -> Result<()>;

    fn get_envelope(&mut self, folder: &str, id: &str) -> Result<Envelope>;
    fn list_envelopes(&mut self, folder: &str, page_size: usize, page: usize) -> Result<Envelopes>;
    fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;
    fn preview_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Messages>;
    fn get_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Messages>;
    fn copy_emails(&mut self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn move_emails(&mut self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn delete_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<()>;

    fn add_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn set_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn remove_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;

    fn mark_emails_as_deleted(&mut self, folder: &str, ids: Vec<&str>) -> Result<()> {
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Deleted]))
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendBuilder {
    account_config: AccountConfig,
    default_credentials: Option<String>,
    disable_cache: bool,
}

impl BackendBuilder {
    pub fn new(account_config: AccountConfig) -> Self {
        Self {
            account_config,
            ..Default::default()
        }
    }

    pub fn with_cache_disabled(mut self, disable_cache: bool) -> Self {
        self.disable_cache = disable_cache;
        self
    }

    pub fn with_default_credentials(mut self) -> Result<Self> {
        self.default_credentials = match &self.account_config.backend {
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Some(imap_config.build_credentials()?)
            }
            _ => None,
        };
        Ok(self)
    }

    pub fn disable_cache(&mut self, disable_cache: bool) {
        self.disable_cache = disable_cache;
    }

    pub fn build(&self) -> Result<Box<dyn Backend>> {
        match &self.account_config.backend {
            BackendConfig::None => Err(Error::BuildBackendError),
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Ok(Box::new(ImapBackend::new(
                    self.account_config.clone(),
                    imap_config.clone(),
                    self.default_credentials.clone(),
                )?))
            }
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(_) => {
                let root_dir = self.account_config.sync_dir()?;
                Ok(Box::new(MaildirBackend::new(
                    self.account_config.clone(),
                    MaildirConfig { root_dir },
                )?))
            }
            BackendConfig::Maildir(mdir_config) => Ok(Box::new(MaildirBackend::new(
                self.account_config.clone(),
                mdir_config.clone(),
            )?)),
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(notmuch_config) => Ok(Box::new(NotmuchBackend::new(
                self.account_config.clone(),
                notmuch_config.clone(),
            )?)),
        }
    }

    pub fn into_build(self) -> Result<Box<dyn Backend>> {
        match self.account_config.backend.clone() {
            BackendConfig::None => Err(Error::BuildBackendError),
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Ok(Box::new(ImapBackend::new(
                    self.account_config,
                    imap_config,
                    self.default_credentials,
                )?))
            }
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(_) => {
                let root_dir = self.account_config.sync_dir()?;
                Ok(Box::new(MaildirBackend::new(
                    self.account_config,
                    MaildirConfig { root_dir },
                )?))
            }
            BackendConfig::Maildir(mdir_config) => Ok(Box::new(MaildirBackend::new(
                self.account_config,
                mdir_config,
            )?)),
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(notmuch_config) => Ok(Box::new(NotmuchBackend::new(
                self.account_config,
                notmuch_config,
            )?)),
        }
    }
}
