//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the [account
//! configuration](crate::AccountConfig).

mod config;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;

use async_trait::async_trait;
use log::error;
use std::{any::Any, sync::Arc};
use thiserror::Error;

use crate::{
    account::AccountConfig,
    email::{Envelope, Envelopes, Flag, Flags, Messages},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder, Folders,
    },
    Result,
};

#[doc(inline)]
pub use self::config::BackendConfig;
#[cfg(feature = "imap-backend")]
#[doc(inline)]
pub use self::imap::{ImapAuthConfig, ImapBackend, ImapConfig};
#[doc(inline)]
pub use self::maildir::{MaildirBackend, MaildirBackendBuilder, MaildirConfig};
#[cfg(feature = "notmuch-backend")]
#[doc(inline)]
pub use self::notmuch::{NotmuchBackend, NotmuchBackendBuilder, NotmuchConfig};

/// Errors related to backend.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build undefined backend")]
    BuildUndefinedBackendError,
    #[error("cannot add folder: feature not available")]
    AddFolderNotAvailableError,
    #[error("cannot list folders: feature not available")]
    ListFoldersNotAvailableError,
    #[error("cannot expunge folder: feature not available")]
    ExpungeFolderNotAvailableError,
    #[error("cannot purge folder: feature not available")]
    PurgeFolderNotAvailableError,
    #[error("cannot delete folder: feature not available")]
    DeleteFolderNotAvailableError,
}

/// The backend abstraction.
///
/// The backend trait abstracts every action needed to manipulate
/// emails.
#[async_trait]
pub trait Backend: Send {
    /// Returns the name of the backend.
    fn name(&self) -> String;

    /// Creates the given folder.
    async fn add_folder(&mut self, folder: &str) -> Result<()>;

    /// Lists all available folders.
    async fn list_folders(&mut self) -> Result<Folders>;

    /// Expunges the given folder.
    ///
    /// The concept is similar to the IMAP expunge: it definitely
    /// deletes emails that have the Deleted flag.
    async fn expunge_folder(&mut self, folder: &str) -> Result<()>;

    /// Purges the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are definitely deleted.
    async fn purge_folder(&mut self, folder: &str) -> Result<()>;

    /// Definitely deletes the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are also definitely deleted.
    async fn delete_folder(&mut self, folder: &str) -> Result<()>;

    /// Gets the envelope from the given folder matching the given id.
    async fn get_envelope(&mut self, folder: &str, id: &str) -> Result<Envelope>;

    /// Lists all available envelopes from the given folder matching
    /// the given pagination.
    async fn list_envelopes(
        &mut self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    /// Sorts and filters envelopes from the given folder matching the
    /// given query, sort and pagination.
    // TODO: we should avoid using strings for query and sort, instead
    // it would be better to have a shared API.
    // See https://todo.sr.ht/~soywod/pimalaya/39.
    async fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    /// Adds the given raw email with the given flags to the given
    /// folder.
    async fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;

    /// Previews emails from the given folder matching the given ids.
    ///
    /// Same as `get_emails`, except that it just "previews": the Seen
    /// flag is not applied to the corresponding envelopes.
    async fn preview_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Messages>;

    /// Gets emails from the given folder matching the given ids.
    async fn get_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Messages>;

    /// Copies emails from the given folder to the given folder
    /// matching the given ids.
    async fn copy_emails(
        &mut self,
        from_folder: &str,
        to_folder: &str,
        ids: Vec<&str>,
    ) -> Result<()>;

    /// Moves emails from the given folder to the given folder
    /// matching the given ids.
    async fn move_emails(
        &mut self,
        from_folder: &str,
        to_folder: &str,
        ids: Vec<&str>,
    ) -> Result<()>;

    /// Deletes emails from the given folder matching the given ids.
    ///
    /// In fact the matching emails are not deleted, they are moved to
    /// the trash folder. If the given folder IS the trash folder,
    /// then it adds the Deleted flag instead. Matching emails will be
    /// definitely deleted after calling `expunge_folder`.
    async fn delete_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<()>;

    /// Adds the given flags to envelopes matching the given ids from
    /// the given folder.
    async fn add_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    /// Replaces envelopes flags matching the given ids from the given
    /// folder.
    async fn set_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    /// Removes the given flags to envelopes matching the given ids
    /// from the given folder.
    async fn remove_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;

    /// Alias for adding the Deleted flag to the matching envelopes.
    async fn mark_emails_as_deleted(&mut self, folder: &str, ids: Vec<&str>) -> Result<()> {
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Deleted]))
            .await
    }

    /// Cleans up sessions, clients, cache etc.
    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn as_any(&self) -> &dyn Any;
}

#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
    type Context: Send + Sync;

    async fn build(self) -> Result<Self::Context>;
}

#[async_trait]
impl BackendContextBuilder for () {
    type Context = ();

    async fn build(self) -> Result<Self::Context> {
        Ok(())
    }
}

pub struct BackendBuilderV2<B: BackendContextBuilder> {
    pub context_builder: B,

    add_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn AddFolder> + Send + Sync>>,
    list_folders: Option<Arc<dyn Fn(&B::Context) -> Box<dyn ListFolders> + Send + Sync>>,
    expunge_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn ExpungeFolder> + Send + Sync>>,
    purge_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn PurgeFolder> + Send + Sync>>,
    delete_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn DeleteFolder> + Send + Sync>>,
}

impl<C, B: BackendContextBuilder<Context = C>> BackendBuilderV2<B> {
    pub fn new(context_builder: B) -> Self {
        Self {
            context_builder,
            add_folder: None,
            list_folders: None,
            expunge_folder: None,
            purge_folder: None,
            delete_folder: None,
        }
    }

    pub fn with_add_folder(
        mut self,
        feature: impl Fn(&C) -> Box<dyn AddFolder> + Send + Sync + 'static,
    ) -> Self {
        self.add_folder = Some(Arc::new(feature));
        self
    }

    pub fn with_list_folders(
        mut self,
        feature: impl Fn(&C) -> Box<dyn ListFolders> + Send + Sync + 'static,
    ) -> Self {
        self.list_folders = Some(Arc::new(feature));
        self
    }

    pub fn with_expunge_folder(
        mut self,
        feature: impl Fn(&C) -> Box<dyn ExpungeFolder> + Send + Sync + 'static,
    ) -> Self {
        self.expunge_folder = Some(Arc::new(feature));
        self
    }

    pub fn with_purge_folder(
        mut self,
        feature: impl Fn(&C) -> Box<dyn PurgeFolder> + Send + Sync + 'static,
    ) -> Self {
        self.purge_folder = Some(Arc::new(feature));
        self
    }

    pub fn with_delete_folder(
        mut self,
        feature: impl Fn(&C) -> Box<dyn DeleteFolder> + Send + Sync + 'static,
    ) -> Self {
        self.delete_folder = Some(Arc::new(feature));
        self
    }

    pub async fn build(self) -> Result<BackendV2<C>> {
        let context = self.context_builder.build().await?;
        let mut backend = BackendV2::new(context);

        if let Some(feature) = self.add_folder {
            backend.set_add_folder(feature(&backend.context));
        }

        if let Some(feature) = self.list_folders {
            backend.set_list_folders(feature(&backend.context));
        }

        if let Some(feature) = self.expunge_folder {
            backend.set_expunge_folder(feature(&backend.context));
        }

        if let Some(feature) = self.purge_folder {
            backend.set_purge_folder(feature(&backend.context));
        }

        if let Some(feature) = self.delete_folder {
            backend.set_delete_folder(feature(&backend.context));
        }

        Ok(backend)
    }
}

impl<B: BackendContextBuilder> Clone for BackendBuilderV2<B> {
    fn clone(&self) -> Self {
        Self {
            context_builder: self.context_builder.clone(),
            add_folder: self.add_folder.clone(),
            list_folders: self.list_folders.clone(),
            expunge_folder: self.expunge_folder.clone(),
            purge_folder: self.purge_folder.clone(),
            delete_folder: self.delete_folder.clone(),
        }
    }
}

impl Default for BackendBuilderV2<()> {
    fn default() -> Self {
        Self {
            context_builder: (),
            add_folder: None,
            list_folders: None,
            expunge_folder: None,
            purge_folder: None,
            delete_folder: None,
        }
    }
}

pub struct BackendV2<C> {
    context: C,

    pub add_folder: Option<Box<dyn AddFolder>>,
    pub list_folders: Option<Box<dyn ListFolders>>,
    pub expunge_folder: Option<Box<dyn ExpungeFolder>>,
    pub purge_folder: Option<Box<dyn PurgeFolder>>,
    pub delete_folder: Option<Box<dyn DeleteFolder>>,
}

impl<C> BackendV2<C> {
    pub fn new(context: C) -> BackendV2<C> {
        BackendV2 {
            context,
            add_folder: None,
            list_folders: None,
            expunge_folder: None,
            purge_folder: None,
            delete_folder: None,
        }
    }

    pub fn set_add_folder(&mut self, feature: Box<dyn AddFolder>) {
        self.add_folder = Some(feature);
    }

    pub fn set_list_folders(&mut self, feature: Box<dyn ListFolders>) {
        self.list_folders = Some(feature);
    }

    pub fn set_expunge_folder(&mut self, feature: Box<dyn ExpungeFolder>) {
        self.expunge_folder = Some(feature);
    }

    pub fn set_purge_folder(&mut self, feature: Box<dyn PurgeFolder>) {
        self.purge_folder = Some(feature);
    }

    pub fn set_delete_folder(&mut self, feature: Box<dyn DeleteFolder>) {
        self.delete_folder = Some(feature);
    }

    pub async fn add_folder(&self, folder: &str) -> Result<()> {
        self.add_folder
            .as_ref()
            .ok_or(Error::AddFolderNotAvailableError)?
            .add_folder(folder)
            .await
    }

    pub async fn list_folders(&self) -> Result<Folders> {
        self.list_folders
            .as_ref()
            .ok_or(Error::ListFoldersNotAvailableError)?
            .list_folders()
            .await
    }

    pub async fn expunge_folder(&self, folder: &str) -> Result<()> {
        self.expunge_folder
            .as_ref()
            .ok_or(Error::ExpungeFolderNotAvailableError)?
            .expunge_folder(folder)
            .await
    }

    pub async fn purge_folder(&self, folder: &str) -> Result<()> {
        self.purge_folder
            .as_ref()
            .ok_or(Error::PurgeFolderNotAvailableError)?
            .purge_folder(folder)
            .await
    }

    pub async fn delete_folder(&self, folder: &str) -> Result<()> {
        self.delete_folder
            .as_ref()
            .ok_or(Error::DeleteFolderNotAvailableError)?
            .delete_folder(folder)
            .await
    }
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
    pub async fn with_default_credentials(mut self) -> Result<Self> {
        self.default_credentials = match &self.account_config.backend {
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Some(imap_config.build_credentials().await?)
            }
            _ => None,
        };
        Ok(self)
    }

    /// Builds a [Backend] by cloning self options.
    pub async fn build(&self) -> Result<Box<dyn Backend>> {
        match &self.account_config.backend {
            BackendConfig::None => Ok(Err(Error::BuildUndefinedBackendError)?),
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Ok(Box::new(
                    ImapBackend::new(
                        self.account_config.clone(),
                        imap_config.clone(),
                        self.default_credentials.clone(),
                    )
                    .await?,
                ))
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

    /// Builds a [Backend] by moving self options.
    pub async fn into_build(self) -> Result<Box<dyn Backend>> {
        match self.account_config.backend.clone() {
            BackendConfig::None => Ok(Err(Error::BuildUndefinedBackendError)?),
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Ok(Box::new(
                    ImapBackend::new(self.account_config, imap_config, self.default_credentials)
                        .await?,
                ))
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
