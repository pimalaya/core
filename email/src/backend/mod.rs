//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the
//! [AccountConfiguration](crate::account::AccountConfig).

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
    email::{
        envelope::{get::GetEnvelope, Id, ListEnvelopes, SingleId},
        flag::{AddFlags, RemoveFlags, SetFlags},
        message::{
            get::default_get_messages, AddRawMessageWithFlags, CopyMessages, DefaultDeleteMessages,
            DeleteMessages, GetMessages, MoveMessages, PeekMessages, SendRawMessage,
        },
        Envelope, Envelopes, Flag, Flags, Messages,
    },
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

    #[error("cannot list envelopes: feature not available")]
    ListEnvelopesNotAvailableError,
    #[error("cannot get envelope: feature not available")]
    GetEnvelopeNotAvailableError,

    #[error("cannot add flag(s): feature not available")]
    AddFlagsNotAvailableError,
    #[error("cannot set flag(s): feature not available")]
    SetFlagsNotAvailableError,
    #[error("cannot remove flag(s): feature not available")]
    RemoveFlagsNotAvailableError,

    #[error("cannot add raw message with flags: feature not available")]
    AddRawMessageWithFlagsNotAvailableError,
    #[error("cannot get messages: feature not available")]
    GetMessagesNotAvailableError,
    #[error("cannot peek messages: feature not available")]
    PeekMessagesNotAvailableError,
    #[error("cannot copy messages: feature not available")]
    CopyMessagesNotAvailableError,
    #[error("cannot move messages: feature not available")]
    MoveMessagesNotAvailableError,
    #[error("cannot delete messages: feature not available")]
    DeleteMessagesNotAvailableError,
    #[error("cannot send raw message: feature not available")]
    SendRawMessageNotAvailableError,
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

#[async_trait]
impl<T: BackendContextBuilder, U: BackendContextBuilder> BackendContextBuilder for (T, U) {
    type Context = (T::Context, U::Context);

    async fn build(self) -> Result<Self::Context> {
        Ok((self.0.build().await?, self.1.build().await?))
    }
}

#[async_trait]
impl<T: BackendContextBuilder, U: BackendContextBuilder, V: BackendContextBuilder>
    BackendContextBuilder for (T, U, V)
{
    type Context = (T::Context, U::Context, V::Context);

    async fn build(self) -> Result<Self::Context> {
        Ok((
            self.0.build().await?,
            self.1.build().await?,
            self.2.build().await?,
        ))
    }
}

pub struct BackendBuilderV2<B: BackendContextBuilder> {
    pub account_config: AccountConfig,

    pub context_builder: B,

    add_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn AddFolder> + Send + Sync>>,
    list_folders: Option<Arc<dyn Fn(&B::Context) -> Box<dyn ListFolders> + Send + Sync>>,
    expunge_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn ExpungeFolder> + Send + Sync>>,
    purge_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn PurgeFolder> + Send + Sync>>,
    delete_folder: Option<Arc<dyn Fn(&B::Context) -> Box<dyn DeleteFolder> + Send + Sync>>,

    list_envelopes: Option<Arc<dyn Fn(&B::Context) -> Box<dyn ListEnvelopes> + Send + Sync>>,
    get_envelope: Option<Arc<dyn Fn(&B::Context) -> Box<dyn GetEnvelope> + Send + Sync>>,

    add_flags: Option<Arc<dyn Fn(&B::Context) -> Box<dyn AddFlags> + Send + Sync>>,
    set_flags: Option<Arc<dyn Fn(&B::Context) -> Box<dyn SetFlags> + Send + Sync>>,
    remove_flags: Option<Arc<dyn Fn(&B::Context) -> Box<dyn RemoveFlags> + Send + Sync>>,

    add_raw_message_with_flags:
        Option<Arc<dyn Fn(&B::Context) -> Box<dyn AddRawMessageWithFlags> + Send + Sync>>,
    get_messages: Option<Arc<dyn Fn(&B::Context) -> Box<dyn GetMessages> + Send + Sync>>,
    peek_messages: Option<Arc<dyn Fn(&B::Context) -> Box<dyn PeekMessages> + Send + Sync>>,
    copy_messages: Option<Arc<dyn Fn(&B::Context) -> Box<dyn CopyMessages> + Send + Sync>>,
    move_messages: Option<Arc<dyn Fn(&B::Context) -> Box<dyn MoveMessages> + Send + Sync>>,
    delete_messages: Option<Arc<dyn Fn(&B::Context) -> Box<dyn DeleteMessages> + Send + Sync>>,
    send_raw_message: Option<Arc<dyn Fn(&B::Context) -> Box<dyn SendRawMessage> + Send + Sync>>,
}

impl<C, B: BackendContextBuilder<Context = C>> BackendBuilderV2<B> {
    pub fn new(account_config: AccountConfig, context_builder: B) -> Self {
        Self {
            account_config,

            context_builder,

            add_folder: Default::default(),
            list_folders: Default::default(),
            expunge_folder: Default::default(),
            purge_folder: Default::default(),
            delete_folder: Default::default(),

            get_envelope: Default::default(),
            list_envelopes: Default::default(),

            add_flags: Default::default(),
            set_flags: Default::default(),
            remove_flags: Default::default(),

            add_raw_message_with_flags: Default::default(),
            get_messages: Default::default(),
            peek_messages: Default::default(),
            copy_messages: Default::default(),
            move_messages: Default::default(),
            delete_messages: Default::default(),
            send_raw_message: Default::default(),
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

    pub fn with_list_envelopes(
        mut self,
        feature: impl Fn(&C) -> Box<dyn ListEnvelopes> + Send + Sync + 'static,
    ) -> Self {
        self.list_envelopes = Some(Arc::new(feature));
        self
    }
    pub fn with_get_envelope(
        mut self,
        feature: impl Fn(&C) -> Box<dyn GetEnvelope> + Send + Sync + 'static,
    ) -> Self {
        self.get_envelope = Some(Arc::new(feature));
        self
    }

    pub fn with_add_flags(
        mut self,
        feature: impl Fn(&C) -> Box<dyn AddFlags> + Send + Sync + 'static,
    ) -> Self {
        self.add_flags = Some(Arc::new(feature));
        self
    }
    pub fn with_set_flags(
        mut self,
        feature: impl Fn(&C) -> Box<dyn SetFlags> + Send + Sync + 'static,
    ) -> Self {
        self.set_flags = Some(Arc::new(feature));
        self
    }
    pub fn with_remove_flags(
        mut self,
        feature: impl Fn(&C) -> Box<dyn RemoveFlags> + Send + Sync + 'static,
    ) -> Self {
        self.remove_flags = Some(Arc::new(feature));
        self
    }

    pub fn with_add_raw_message_with_flags(
        mut self,
        feature: impl Fn(&C) -> Box<dyn AddRawMessageWithFlags> + Send + Sync + 'static,
    ) -> Self {
        self.add_raw_message_with_flags = Some(Arc::new(feature));
        self
    }
    pub fn with_get_messages(
        mut self,
        feature: impl Fn(&C) -> Box<dyn GetMessages> + Send + Sync + 'static,
    ) -> Self {
        self.get_messages = Some(Arc::new(feature));
        self
    }
    pub fn with_peek_messages(
        mut self,
        feature: impl Fn(&C) -> Box<dyn PeekMessages> + Send + Sync + 'static,
    ) -> Self {
        self.peek_messages = Some(Arc::new(feature));
        self
    }
    pub fn with_copy_messages(
        mut self,
        feature: impl Fn(&C) -> Box<dyn CopyMessages> + Send + Sync + 'static,
    ) -> Self {
        self.copy_messages = Some(Arc::new(feature));
        self
    }
    pub fn with_move_messages(
        mut self,
        feature: impl Fn(&C) -> Box<dyn MoveMessages> + Send + Sync + 'static,
    ) -> Self {
        self.move_messages = Some(Arc::new(feature));
        self
    }
    pub fn with_delete_messages(
        mut self,
        feature: impl Fn(&C) -> Box<dyn DeleteMessages> + Send + Sync + 'static,
    ) -> Self {
        self.delete_messages = Some(Arc::new(feature));
        self
    }
    pub fn with_send_raw_message(
        mut self,
        feature: impl Fn(&C) -> Box<dyn SendRawMessage> + Send + Sync + 'static,
    ) -> Self {
        self.send_raw_message = Some(Arc::new(feature));
        self
    }

    pub async fn build(self) -> Result<BackendV2<C>> {
        let context = self.context_builder.build().await?;
        let mut backend = BackendV2::new(self.account_config.clone(), context);

        if let Some(feature) = &self.add_folder {
            backend.set_add_folder(feature(&backend.context));
        }
        if let Some(feature) = &self.list_folders {
            backend.set_list_folders(feature(&backend.context));
        }
        if let Some(feature) = &self.expunge_folder {
            backend.set_expunge_folder(feature(&backend.context));
        }
        if let Some(feature) = &self.purge_folder {
            backend.set_purge_folder(feature(&backend.context));
        }
        if let Some(feature) = &self.delete_folder {
            backend.set_delete_folder(feature(&backend.context));
        }

        if let Some(feature) = &self.list_envelopes {
            backend.set_list_envelopes(feature(&backend.context));
        }
        if let Some(feature) = &self.get_envelope {
            backend.set_get_envelope(feature(&backend.context));
        }

        if let Some(feature) = &self.add_flags {
            backend.set_add_flags(feature(&backend.context));
        }
        if let Some(feature) = &self.set_flags {
            backend.set_set_flags(feature(&backend.context));
        }
        if let Some(feature) = &self.remove_flags {
            backend.set_remove_flags(feature(&backend.context));
        }

        if let Some(feature) = &self.add_raw_message_with_flags {
            backend.set_add_raw_message_with_flags(feature(&backend.context));
        }
        if let Some(feature) = &self.get_messages {
            backend.set_get_messages(feature(&backend.context));
        }
        if let Some(feature) = &self.peek_messages {
            backend.set_peek_messages(feature(&backend.context));
        }
        if let Some(feature) = &self.copy_messages {
            backend.set_copy_messages(feature(&backend.context));
        }
        if let Some(feature) = &self.move_messages {
            backend.set_move_messages(feature(&backend.context));
        }
        if let Some(feature) = &self.delete_messages {
            backend.set_delete_messages(feature(&backend.context));
        } else if let (Some(a), Some(b)) = (&self.move_messages, &self.add_flags) {
            backend.set_delete_messages(DefaultDeleteMessages::new(
                self.account_config.clone(),
                a(&backend.context),
                b(&backend.context),
            ))
        }
        if let Some(feature) = self.send_raw_message {
            backend.set_send_raw_message(feature(&backend.context));
        }

        Ok(backend)
    }
}

impl<B: BackendContextBuilder> Clone for BackendBuilderV2<B> {
    fn clone(&self) -> Self {
        Self {
            context_builder: self.context_builder.clone(),

            account_config: self.account_config.clone(),

            add_folder: self.add_folder.clone(),
            list_folders: self.list_folders.clone(),
            expunge_folder: self.expunge_folder.clone(),
            purge_folder: self.purge_folder.clone(),
            delete_folder: self.delete_folder.clone(),

            list_envelopes: self.list_envelopes.clone(),
            get_envelope: self.get_envelope.clone(),

            add_flags: self.add_flags.clone(),
            set_flags: self.set_flags.clone(),
            remove_flags: self.remove_flags.clone(),

            add_raw_message_with_flags: self.add_raw_message_with_flags.clone(),
            get_messages: self.get_messages.clone(),
            peek_messages: self.peek_messages.clone(),
            copy_messages: self.copy_messages.clone(),
            move_messages: self.move_messages.clone(),
            delete_messages: self.delete_messages.clone(),
            send_raw_message: self.send_raw_message.clone(),
        }
    }
}

impl Default for BackendBuilderV2<()> {
    fn default() -> Self {
        Self {
            context_builder: (),

            account_config: Default::default(),

            add_folder: Default::default(),
            list_folders: Default::default(),
            expunge_folder: Default::default(),
            purge_folder: Default::default(),
            delete_folder: Default::default(),

            add_flags: Default::default(),
            set_flags: Default::default(),
            remove_flags: Default::default(),

            list_envelopes: Default::default(),
            get_envelope: Default::default(),

            add_raw_message_with_flags: Default::default(),
            get_messages: Default::default(),
            peek_messages: Default::default(),
            copy_messages: Default::default(),
            move_messages: Default::default(),
            delete_messages: Default::default(),
            send_raw_message: Default::default(),
        }
    }
}

pub struct BackendV2<C> {
    context: C,

    pub account_config: AccountConfig,

    pub add_folder: Option<Box<dyn AddFolder>>,
    pub list_folders: Option<Box<dyn ListFolders>>,
    pub expunge_folder: Option<Box<dyn ExpungeFolder>>,
    pub purge_folder: Option<Box<dyn PurgeFolder>>,
    pub delete_folder: Option<Box<dyn DeleteFolder>>,

    pub list_envelopes: Option<Box<dyn ListEnvelopes>>,
    pub get_envelope: Option<Box<dyn GetEnvelope>>,

    pub add_flags: Option<Box<dyn AddFlags>>,
    pub set_flags: Option<Box<dyn SetFlags>>,
    pub remove_flags: Option<Box<dyn RemoveFlags>>,

    pub add_raw_message_with_flags: Option<Box<dyn AddRawMessageWithFlags>>,
    pub get_messages: Option<Box<dyn GetMessages>>,
    pub peek_messages: Option<Box<dyn PeekMessages>>,
    pub copy_messages: Option<Box<dyn CopyMessages>>,
    pub move_messages: Option<Box<dyn MoveMessages>>,
    pub delete_messages: Option<Box<dyn DeleteMessages>>,
    pub send_raw_message: Option<Box<dyn SendRawMessage>>,
}

impl<C> BackendV2<C> {
    pub fn new(account_config: AccountConfig, context: C) -> BackendV2<C> {
        BackendV2 {
            context,

            account_config,

            add_folder: None,
            list_folders: None,
            expunge_folder: None,
            purge_folder: None,
            delete_folder: None,

            get_envelope: None,
            list_envelopes: None,

            add_flags: None,
            set_flags: None,
            remove_flags: None,

            add_raw_message_with_flags: None,
            get_messages: None,
            peek_messages: None,
            copy_messages: None,
            move_messages: None,
            delete_messages: None,
            send_raw_message: None,
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

    pub fn set_list_envelopes(&mut self, feature: Box<dyn ListEnvelopes>) {
        self.list_envelopes = Some(feature);
    }
    pub fn set_get_envelope(&mut self, feature: Box<dyn GetEnvelope>) {
        self.get_envelope = Some(feature);
    }

    pub fn set_add_flags(&mut self, feature: Box<dyn AddFlags>) {
        self.add_flags = Some(feature);
    }
    pub fn set_set_flags(&mut self, feature: Box<dyn SetFlags>) {
        self.set_flags = Some(feature);
    }
    pub fn set_remove_flags(&mut self, feature: Box<dyn RemoveFlags>) {
        self.remove_flags = Some(feature);
    }

    pub fn set_add_raw_message_with_flags(&mut self, feature: Box<dyn AddRawMessageWithFlags>) {
        self.add_raw_message_with_flags = Some(feature);
    }
    pub fn set_get_messages(&mut self, feature: Box<dyn GetMessages>) {
        self.get_messages = Some(feature);
    }
    pub fn set_peek_messages(&mut self, feature: Box<dyn PeekMessages>) {
        self.peek_messages = Some(feature);
    }
    pub fn set_copy_messages(&mut self, feature: Box<dyn CopyMessages>) {
        self.copy_messages = Some(feature);
    }
    pub fn set_move_messages(&mut self, feature: Box<dyn MoveMessages>) {
        self.move_messages = Some(feature);
    }
    pub fn set_delete_messages(&mut self, feature: Box<dyn DeleteMessages>) {
        self.delete_messages = Some(feature);
    }
    pub fn set_send_raw_message(&mut self, feature: Box<dyn SendRawMessage>) {
        self.send_raw_message = Some(feature);
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

    pub async fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes> {
        self.list_envelopes
            .as_ref()
            .ok_or(Error::ListEnvelopesNotAvailableError)?
            .list_envelopes(folder, page_size, page)
            .await
    }

    pub async fn get_envelope(&self, folder: &str, id: &str) -> Result<Envelope> {
        self.get_envelope
            .as_ref()
            .ok_or(Error::GetEnvelopeNotAvailableError)?
            .get_envelope(folder, id)
            .await
    }

    pub async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.add_flags
            .as_ref()
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flags(folder, id, flags)
            .await
    }

    pub async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.add_flags
            .as_ref()
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flag(folder, id, flag)
            .await
    }

    pub async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.set_flags
            .as_ref()
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flags(folder, id, flags)
            .await
    }

    pub async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flags(folder, id, flags)
            .await
    }

    pub async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        email: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        self.add_raw_message_with_flags
            .as_ref()
            .ok_or(Error::AddRawMessageWithFlagsNotAvailableError)?
            .add_raw_message_with_flags(folder, email, flags)
            .await
    }

    pub async fn add_raw_message_with_flag(
        &self,
        folder: &str,
        email: &[u8],
        flag: Flag,
    ) -> Result<SingleId> {
        self.add_raw_message_with_flags
            .as_ref()
            .ok_or(Error::AddRawMessageWithFlagsNotAvailableError)?
            .add_raw_message_with_flag(folder, email, flag)
            .await
    }

    pub async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        if let Some(f) = self.get_messages.as_ref() {
            f.get_messages(folder, id).await
        } else if let (Some(a), Some(b)) = (self.peek_messages.as_ref(), self.add_flags.as_ref()) {
            default_get_messages(a.as_ref(), b.as_ref(), folder, id).await
        } else {
            Err(Error::PeekMessagesNotAvailableError)?
        }
    }

    pub async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        self.peek_messages
            .as_ref()
            .ok_or(Error::PeekMessagesNotAvailableError)?
            .peek_messages(folder, id)
            .await
    }

    pub async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.copy_messages
            .as_ref()
            .ok_or(Error::CopyMessagesNotAvailableError)?
            .copy_messages(from_folder, to_folder, id)
            .await
    }

    pub async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.move_messages
            .as_ref()
            .ok_or(Error::MoveMessagesNotAvailableError)?
            .move_messages(from_folder, to_folder, id)
            .await
    }

    pub async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        self.delete_messages
            .as_ref()
            .ok_or(Error::DeleteMessagesNotAvailableError)?
            .delete_messages(folder, id)
            .await
    }

    pub async fn send_raw_message(&self, raw_msg: &[u8]) -> Result<()> {
        self.send_raw_message
            .as_ref()
            .ok_or(Error::SendRawMessageNotAvailableError)?
            .send_raw_message(raw_msg)
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
