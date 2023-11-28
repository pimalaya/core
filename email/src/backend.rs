//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the
//! [AccountConfiguration](crate::account::AccountConfig).

use async_trait::async_trait;
use log::error;
use std::sync::Arc;
use thiserror::Error;

use crate::{
    account::AccountConfig,
    email::{
        envelope::{get::GetEnvelope, list::ListEnvelopes, Id, SingleId},
        flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags},
        message::{
            add_raw::AddRawMessage, add_raw_with_flags::AddRawMessageWithFlags, copy::CopyMessages,
            delete::default_delete_messages, delete::DeleteMessages, get::default_get_messages,
            get::GetMessages, move_::MoveMessages, peek::PeekMessages, send_raw::SendRawMessage,
        },
        Envelope, Envelopes, Flag, Flags, Messages,
    },
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder, Folders,
    },
    Result,
};

/// Errors related to backend.
#[derive(Debug, Error)]
pub enum Error {
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

    #[error("cannot add raw message: feature not available")]
    AddRawMessageNotAvailableError,
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

pub struct BackendBuilder<B: BackendContextBuilder> {
    pub account_config: AccountConfig,

    pub context_builder: B,

    add_folder: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddFolder>> + Send + Sync>>,
    list_folders: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn ListFolders>> + Send + Sync>>,
    expunge_folder:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn ExpungeFolder>> + Send + Sync>>,
    purge_folder: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn PurgeFolder>> + Send + Sync>>,
    delete_folder: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn DeleteFolder>> + Send + Sync>>,

    list_envelopes:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn ListEnvelopes>> + Send + Sync>>,
    get_envelope: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn GetEnvelope>> + Send + Sync>>,

    add_flags: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddFlags>> + Send + Sync>>,
    set_flags: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn SetFlags>> + Send + Sync>>,
    remove_flags: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn RemoveFlags>> + Send + Sync>>,

    add_raw_message:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddRawMessage>> + Send + Sync>>,
    add_raw_message_with_flags:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddRawMessageWithFlags>> + Send + Sync>>,
    get_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn GetMessages>> + Send + Sync>>,
    peek_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn PeekMessages>> + Send + Sync>>,
    copy_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn CopyMessages>> + Send + Sync>>,
    move_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn MoveMessages>> + Send + Sync>>,
    delete_messages:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn DeleteMessages>> + Send + Sync>>,
    send_raw_message:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn SendRawMessage>> + Send + Sync>>,
}

impl<C, B: BackendContextBuilder<Context = C>> BackendBuilder<B> {
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

            add_raw_message: Default::default(),
            add_raw_message_with_flags: Default::default(),
            get_messages: Default::default(),
            peek_messages: Default::default(),
            copy_messages: Default::default(),
            move_messages: Default::default(),
            delete_messages: Default::default(),
            send_raw_message: Default::default(),
        }
    }

    pub fn set_add_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFolder>> + Send + Sync + 'static,
    ) {
        self.add_folder = Some(Arc::new(feature));
    }
    pub fn with_add_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_folder(feature);
        self
    }

    pub fn set_list_folders(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) {
        self.list_folders = Some(Arc::new(feature));
    }
    pub fn with_list_folders(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_folders(feature);
        self
    }

    pub fn set_expunge_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ExpungeFolder>> + Send + Sync + 'static,
    ) {
        self.expunge_folder = Some(Arc::new(feature));
    }
    pub fn with_expunge_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ExpungeFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_expunge_folder(feature);
        self
    }

    pub fn set_purge_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PurgeFolder>> + Send + Sync + 'static,
    ) {
        self.purge_folder = Some(Arc::new(feature));
    }
    pub fn with_purge_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PurgeFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_purge_folder(feature);
        self
    }

    pub fn set_delete_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteFolder>> + Send + Sync + 'static,
    ) {
        self.delete_folder = Some(Arc::new(feature));
    }
    pub fn with_delete_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_delete_folder(feature);
        self
    }

    pub fn set_list_envelopes(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListEnvelopes>> + Send + Sync + 'static,
    ) {
        self.list_envelopes = Some(Arc::new(feature));
    }
    pub fn with_list_envelopes(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListEnvelopes>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_envelopes(feature);
        self
    }

    pub fn set_get_envelope(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetEnvelope>> + Send + Sync + 'static,
    ) {
        self.get_envelope = Some(Arc::new(feature));
    }
    pub fn with_get_envelope(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetEnvelope>> + Send + Sync + 'static,
    ) -> Self {
        self.set_get_envelope(feature);
        self
    }

    pub fn set_add_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFlags>> + Send + Sync + 'static,
    ) {
        self.add_flags = Some(Arc::new(feature));
    }
    pub fn with_add_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_flags(feature);
        self
    }

    pub fn set_set_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SetFlags>> + Send + Sync + 'static,
    ) {
        self.set_flags = Some(Arc::new(feature));
    }
    pub fn with_set_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SetFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_set_flags(feature);
        self
    }
    pub fn set_remove_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn RemoveFlags>> + Send + Sync + 'static,
    ) {
        self.remove_flags = Some(Arc::new(feature));
    }
    pub fn with_remove_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn RemoveFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_remove_flags(feature);
        self
    }

    pub fn set_add_raw_message(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddRawMessage>> + Send + Sync + 'static,
    ) {
        self.add_raw_message = Some(Arc::new(feature));
    }
    pub fn with_add_raw_message(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddRawMessage>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_raw_message(feature);
        self
    }

    pub fn set_add_raw_message_with_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddRawMessageWithFlags>> + Send + Sync + 'static,
    ) {
        self.add_raw_message_with_flags = Some(Arc::new(feature));
    }
    pub fn with_add_raw_message_with_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddRawMessageWithFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_raw_message_with_flags(feature);
        self
    }

    pub fn set_get_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetMessages>> + Send + Sync + 'static,
    ) {
        self.get_messages = Some(Arc::new(feature));
    }
    pub fn with_get_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_get_messages(feature);
        self
    }

    pub fn set_peek_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PeekMessages>> + Send + Sync + 'static,
    ) {
        self.peek_messages = Some(Arc::new(feature));
    }
    pub fn with_peek_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PeekMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_peek_messages(feature);
        self
    }

    pub fn set_copy_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn CopyMessages>> + Send + Sync + 'static,
    ) {
        self.copy_messages = Some(Arc::new(feature));
    }
    pub fn with_copy_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn CopyMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_copy_messages(feature);
        self
    }

    pub fn set_move_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn MoveMessages>> + Send + Sync + 'static,
    ) {
        self.move_messages = Some(Arc::new(feature));
    }
    pub fn with_move_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn MoveMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_move_messages(feature);
        self
    }

    pub fn set_delete_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteMessages>> + Send + Sync + 'static,
    ) {
        self.delete_messages = Some(Arc::new(feature));
    }
    pub fn with_delete_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_delete_messages(feature);
        self
    }

    pub fn set_send_raw_message(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SendRawMessage>> + Send + Sync + 'static,
    ) {
        self.send_raw_message = Some(Arc::new(feature));
    }
    pub fn with_send_raw_message(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SendRawMessage>> + Send + Sync + 'static,
    ) -> Self {
        self.set_send_raw_message(feature);
        self
    }

    pub async fn build(self) -> Result<Backend<C>> {
        let context = self.context_builder.build().await?;
        let mut backend = Backend::new(self.account_config.clone(), context);

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
        }
        if let Some(feature) = self.send_raw_message {
            backend.set_send_raw_message(feature(&backend.context));
        }

        Ok(backend)
    }
}

impl<B: BackendContextBuilder> Clone for BackendBuilder<B> {
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

            add_raw_message: self.add_raw_message.clone(),
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

impl Default for BackendBuilder<()> {
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

            add_raw_message: Default::default(),
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

pub struct Backend<C> {
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

    pub add_raw_message: Option<Box<dyn AddRawMessage>>,
    pub add_raw_message_with_flags: Option<Box<dyn AddRawMessageWithFlags>>,
    pub get_messages: Option<Box<dyn GetMessages>>,
    pub peek_messages: Option<Box<dyn PeekMessages>>,
    pub copy_messages: Option<Box<dyn CopyMessages>>,
    pub move_messages: Option<Box<dyn MoveMessages>>,
    pub delete_messages: Option<Box<dyn DeleteMessages>>,
    pub send_raw_message: Option<Box<dyn SendRawMessage>>,
}

impl<C> Backend<C> {
    pub fn new(account_config: AccountConfig, context: C) -> Backend<C> {
        Backend {
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

            add_raw_message: None,
            add_raw_message_with_flags: None,
            get_messages: None,
            peek_messages: None,
            copy_messages: None,
            move_messages: None,
            delete_messages: None,
            send_raw_message: None,
        }
    }

    pub fn set_add_folder(&mut self, feature: Option<Box<dyn AddFolder>>) {
        self.add_folder = feature;
    }
    pub fn set_list_folders(&mut self, feature: Option<Box<dyn ListFolders>>) {
        self.list_folders = feature;
    }
    pub fn set_expunge_folder(&mut self, feature: Option<Box<dyn ExpungeFolder>>) {
        self.expunge_folder = feature;
    }
    pub fn set_purge_folder(&mut self, feature: Option<Box<dyn PurgeFolder>>) {
        self.purge_folder = feature;
    }
    pub fn set_delete_folder(&mut self, feature: Option<Box<dyn DeleteFolder>>) {
        self.delete_folder = feature;
    }

    pub fn set_list_envelopes(&mut self, feature: Option<Box<dyn ListEnvelopes>>) {
        self.list_envelopes = feature;
    }
    pub fn set_get_envelope(&mut self, feature: Option<Box<dyn GetEnvelope>>) {
        self.get_envelope = feature;
    }

    pub fn set_add_flags(&mut self, feature: Option<Box<dyn AddFlags>>) {
        self.add_flags = feature;
    }
    pub fn set_set_flags(&mut self, feature: Option<Box<dyn SetFlags>>) {
        self.set_flags = feature;
    }
    pub fn set_remove_flags(&mut self, feature: Option<Box<dyn RemoveFlags>>) {
        self.remove_flags = feature;
    }

    pub fn set_add_raw_message_with_flags(
        &mut self,
        feature: Option<Box<dyn AddRawMessageWithFlags>>,
    ) {
        self.add_raw_message_with_flags = feature;
    }
    pub fn set_get_messages(&mut self, feature: Option<Box<dyn GetMessages>>) {
        self.get_messages = feature;
    }
    pub fn set_peek_messages(&mut self, feature: Option<Box<dyn PeekMessages>>) {
        self.peek_messages = feature;
    }
    pub fn set_copy_messages(&mut self, feature: Option<Box<dyn CopyMessages>>) {
        self.copy_messages = feature;
    }
    pub fn set_move_messages(&mut self, feature: Option<Box<dyn MoveMessages>>) {
        self.move_messages = feature;
    }
    pub fn set_delete_messages(&mut self, feature: Option<Box<dyn DeleteMessages>>) {
        self.delete_messages = feature;
    }
    pub fn set_send_raw_message(&mut self, feature: Option<Box<dyn SendRawMessage>>) {
        self.send_raw_message = feature;
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

    pub async fn get_envelope(&self, folder: &str, id: &Id) -> Result<Envelope> {
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

    pub async fn set_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.set_flags
            .as_ref()
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flag(folder, id, flag)
            .await
    }

    pub async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flags(folder, id, flags)
            .await
    }

    pub async fn remove_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flag(folder, id, flag)
            .await
    }

    pub async fn add_raw_message(&self, folder: &str, email: &[u8]) -> Result<SingleId> {
        if let Some(f) = self.add_raw_message.as_ref() {
            f.add_raw_message(folder, email).await
        } else if let Some(f) = self.add_raw_message_with_flags.as_ref() {
            f.add_raw_message_with_flags(folder, email, &Default::default())
                .await
        } else {
            Ok(Err(Error::AddRawMessageNotAvailableError)?)
        }
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
            Ok(Err(Error::PeekMessagesNotAvailableError)?)
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
        if let Some(f) = self.delete_messages.as_ref() {
            f.delete_messages(folder, id).await
        } else if let (Some(a), Some(b)) = (self.move_messages.as_ref(), self.add_flags.as_ref()) {
            default_delete_messages(&self.account_config, a.as_ref(), b.as_ref(), folder, id).await
        } else {
            Ok(Err(Error::DeleteMessagesNotAvailableError)?)
        }
    }

    pub async fn send_raw_message(&self, raw_msg: &[u8]) -> Result<()> {
        self.send_raw_message
            .as_ref()
            .ok_or(Error::SendRawMessageNotAvailableError)?
            .send_raw_message(raw_msg)
            .await
    }
}
