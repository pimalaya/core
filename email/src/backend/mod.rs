//! Module dedicated to backend.
//!
//! The core concept of this module is the [Backend] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [BackendConfig] which represents the
//! backend-specific configuration, mostly used by the
//! [account configuration](crate::AccountConfig).
//!
//! You also have everything you need to
//! [synchronize](crate::backend::sync) a remote backend with a local
//! Maildir backend.

mod config;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;
pub mod sync;

use log::error;
use std::any::Any;
use thiserror::Error;

use crate::{AccountConfig, Envelope, Envelopes, Flag, Flags, Folders, Messages, Result};

#[cfg(feature = "imap-backend")]
pub use self::imap::*;
pub use self::maildir::*;
#[cfg(feature = "notmuch-backend")]
pub use self::notmuch::*;
pub use self::{
    config::BackendConfig,
    sync::{BackendSyncBuilder, BackendSyncProgress, BackendSyncProgressEvent, BackendSyncReport},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build undefined backend")]
    BuildUndefinedBackendError,
}

/// The backend abstraction.
///
/// The backend trait abstracts every action needed to manipulate
/// emails.
pub trait Backend {
    /// Returns the name of the backend.
    fn name(&self) -> String;

    /// Creates the given folder.
    fn add_folder(&mut self, folder: &str) -> Result<()>;

    /// Lists all available folders.
    fn list_folders(&mut self) -> Result<Folders>;

    /// Expunges the given folder.
    ///
    /// The concept is similar to the IMAP expunge: it definitely
    /// deletes emails that have the Deleted flag.
    fn expunge_folder(&mut self, folder: &str) -> Result<()>;

    /// Purges the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are definitely deleted.
    fn purge_folder(&mut self, folder: &str) -> Result<()>;

    /// Definitely deletes the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are also definitely deleted.
    fn delete_folder(&mut self, folder: &str) -> Result<()>;

    /// Gets the envelope from the given folder matching the given id.
    fn get_envelope(&mut self, folder: &str, id: &str) -> Result<Envelope>;

    /// Lists all available envelopes from the given folder matching
    /// the given pagination.
    fn list_envelopes(&mut self, folder: &str, page_size: usize, page: usize) -> Result<Envelopes>;

    /// Sorts and filters envelopes from the given folder matching the
    /// given query, sort and pagination.
    // TODO: we should avoid using strings for query and sort, instead
    // it would be better to have a shared API.
    // See https://todo.sr.ht/~soywod/pimalaya/39.
    fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    /// Adds the given raw email with the given flags to the given
    /// folder.
    fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;

    /// Previews emails from the given folder matching the given ids.
    ///
    /// Same as `get_emails`, except that it just "previews": the Seen
    /// flag is not applied to the corresponding envelopes.
    fn preview_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Messages>;

    /// Gets emails from the given folder matching the given ids.
    fn get_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Messages>;

    /// Copies emails from the given folder to the given folder
    /// matching the given ids.
    fn copy_emails(&mut self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;

    /// Moves emails from the given folder to the given folder
    /// matching the given ids.
    fn move_emails(&mut self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;

    /// Deletes emails from the given folder matching the given ids.
    ///
    /// In fact the matching emails are not deleted, they are moved to
    /// the trash folder. If the given folder IS the trash folder,
    /// then it adds the Deleted flag instead. Matching emails will be
    /// definitely deleted after calling `expunge_folder`.
    fn delete_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<()>;

    /// Adds the given flags to envelopes matching the given ids from
    /// the given folder.
    fn add_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    /// Replaces envelopes flags matching the given ids from the given
    /// folder.
    fn set_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    /// Removes the given flags to envelopes matching the given ids
    /// from the given folder.
    fn remove_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;

    /// Alias for adding the Deleted flag to the matching envelopes.
    fn mark_emails_as_deleted(&mut self, folder: &str, ids: Vec<&str>) -> Result<()> {
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Deleted]))
    }

    /// Cleans up sessions, clients, cache etc.
    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any;
}

/// The backend builder.
///
/// This builder helps you to build a `Box<dyn Backend>`. The type of
/// backend depends on the given account configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendBuilder {
    account_config: AccountConfig,
    default_credentials: Option<String>,
    disable_cache: bool,
}

impl BackendBuilder {
    /// Creates a new builder with default value.
    pub fn new(account_config: AccountConfig) -> Self {
        Self {
            account_config,
            ..Default::default()
        }
    }

    /// Disable cache setter.
    pub fn disable_cache(&mut self, disable_cache: bool) {
        self.disable_cache = disable_cache;
    }

    /// Disable cache setter following the builder pattern.
    pub fn with_cache_disabled(mut self, disable_cache: bool) -> Self {
        self.disable_cache = disable_cache;
        self
    }

    /// Default credentials setter following the builder pattern.
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

    /// Builds a [backend](crate::Backend) by cloning self options.
    pub fn build(&self) -> Result<Box<dyn Backend>> {
        match &self.account_config.backend {
            BackendConfig::None => Ok(Err(Error::BuildUndefinedBackendError)?),
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

    /// Builds a [backend](crate::Backend) by moving self options.
    pub fn into_build(self) -> Result<Box<dyn Backend>> {
        match self.account_config.backend.clone() {
            BackendConfig::None => Ok(Err(Error::BuildUndefinedBackendError)?),
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
