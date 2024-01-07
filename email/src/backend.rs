//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the
//! [AccountConfiguration](crate::account::config::AccountConfig).

use async_trait::async_trait;
#[allow(unused)]
use std::sync::Arc;
use thiserror::Error;

#[cfg(feature = "envelope-watch")]
use crate::envelope::watch::WatchEnvelopes;
#[cfg(feature = "envelope-get")]
use crate::envelope::{get::GetEnvelope, Envelope};
#[cfg(feature = "envelope-list")]
use crate::envelope::{list::ListEnvelopes, Envelopes};
#[cfg(feature = "flag-add")]
use crate::flag::add::AddFlags;
#[cfg(feature = "flag-remove")]
use crate::flag::remove::RemoveFlags;
#[cfg(feature = "flag-set")]
use crate::flag::set::SetFlags;
#[cfg(any(feature = "flag-any", feature = "message-add"))]
use crate::flag::{Flag, Flags};
#[cfg(feature = "folder-add")]
use crate::folder::add::AddFolder;
#[cfg(feature = "folder-delete")]
use crate::folder::delete::DeleteFolder;
#[cfg(feature = "folder-expunge")]
use crate::folder::expunge::ExpungeFolder;
#[cfg(feature = "folder-purge")]
use crate::folder::purge::PurgeFolder;
#[cfg(feature = "folder-list")]
use crate::folder::{list::ListFolders, Folders};
#[cfg(all(feature = "message-add", feature = "flag-add"))]
use crate::message::add_with_flags::{
    default_add_message_with_flag, default_add_message_with_flags,
};
#[cfg(feature = "message-copy")]
use crate::message::copy::CopyMessages;
#[cfg(all(feature = "message-delete", feature = "flag-add"))]
use crate::message::delete::default_delete_messages;
#[cfg(feature = "message-delete")]
use crate::message::delete::DeleteMessages;
#[cfg(all(feature = "message-get", feature = "flag-add"))]
use crate::message::get::default_get_messages;
#[cfg(feature = "message-get")]
use crate::message::get::GetMessages;
#[cfg(feature = "message-move")]
use crate::message::move_::MoveMessages;
#[cfg(feature = "message-peek")]
use crate::message::peek::PeekMessages;
#[cfg(feature = "message-send")]
use crate::message::send::SendMessage;
#[cfg(any(feature = "message-peek", feature = "message-get"))]
use crate::message::Messages;
#[allow(unused)]
use crate::{account::config::AccountConfig, envelope::Id, Result};
#[cfg(feature = "message-add")]
use crate::{
    envelope::SingleId,
    message::{add::AddMessage, add_with_flags::AddMessageWithFlags},
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
    #[error("cannot watch for envelopes changes: feature not available")]
    WatchEnvelopesNotAvailableError,
    #[error("cannot get envelope: feature not available")]
    GetEnvelopeNotAvailableError,

    #[error("cannot add flag(s): feature not available")]
    AddFlagsNotAvailableError,
    #[error("cannot set flag(s): feature not available")]
    SetFlagsNotAvailableError,
    #[error("cannot remove flag(s): feature not available")]
    RemoveFlagsNotAvailableError,

    #[error("cannot add message: feature not available")]
    AddMessageNotAvailableError,
    #[error("cannot add message with flags: feature not available")]
    AddMessageWithFlagsNotAvailableError,
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
    #[error("cannot send message: feature not available")]
    SendMessageNotAvailableError,
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

    #[cfg(feature = "folder-add")]
    add_folder: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddFolder>> + Send + Sync>>,
    #[cfg(feature = "folder-list")]
    list_folders: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn ListFolders>> + Send + Sync>>,
    #[cfg(feature = "folder-expunge")]
    expunge_folder:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn ExpungeFolder>> + Send + Sync>>,
    #[cfg(feature = "folder-purge")]
    purge_folder: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn PurgeFolder>> + Send + Sync>>,
    #[cfg(feature = "folder-delete")]
    delete_folder: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn DeleteFolder>> + Send + Sync>>,

    #[cfg(feature = "envelope-list")]
    list_envelopes:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn ListEnvelopes>> + Send + Sync>>,
    #[cfg(feature = "envelope-watch")]
    watch_envelopes:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn WatchEnvelopes>> + Send + Sync>>,
    #[cfg(feature = "envelope-get")]
    get_envelope: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn GetEnvelope>> + Send + Sync>>,

    #[cfg(feature = "flag-add")]
    add_flags: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddFlags>> + Send + Sync>>,
    #[cfg(feature = "flag-set")]
    set_flags: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn SetFlags>> + Send + Sync>>,
    #[cfg(feature = "flag-remove")]
    remove_flags: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn RemoveFlags>> + Send + Sync>>,

    #[cfg(feature = "message-add")]
    add_message: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddMessage>> + Send + Sync>>,
    #[cfg(feature = "message-add")]
    add_message_with_flags:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddMessageWithFlags>> + Send + Sync>>,
    #[cfg(feature = "message-peek")]
    peek_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn PeekMessages>> + Send + Sync>>,
    #[cfg(feature = "message-get")]
    get_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn GetMessages>> + Send + Sync>>,
    #[cfg(feature = "message-copy")]
    copy_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn CopyMessages>> + Send + Sync>>,
    #[cfg(feature = "message-move")]
    move_messages: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn MoveMessages>> + Send + Sync>>,
    #[cfg(feature = "message-delete")]
    delete_messages:
        Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn DeleteMessages>> + Send + Sync>>,
    #[cfg(feature = "message-send")]
    send_message: Option<Arc<dyn Fn(&B::Context) -> Option<Box<dyn SendMessage>> + Send + Sync>>,
}

impl<C, B: BackendContextBuilder<Context = C>> BackendBuilder<B> {
    pub fn new(account_config: AccountConfig, context_builder: B) -> Self {
        Self {
            account_config,
            context_builder,

            #[cfg(feature = "folder-add")]
            add_folder: Default::default(),
            #[cfg(feature = "folder-list")]
            list_folders: Default::default(),
            #[cfg(feature = "folder-expunge")]
            expunge_folder: Default::default(),
            #[cfg(feature = "folder-purge")]
            purge_folder: Default::default(),
            #[cfg(feature = "folder-delete")]
            delete_folder: Default::default(),

            #[cfg(feature = "envelope-list")]
            list_envelopes: Default::default(),
            #[cfg(feature = "envelope-watch")]
            watch_envelopes: Default::default(),
            #[cfg(feature = "envelope-get")]
            get_envelope: Default::default(),

            #[cfg(feature = "flag-add")]
            add_flags: Default::default(),
            #[cfg(feature = "flag-set")]
            set_flags: Default::default(),
            #[cfg(feature = "flag-remove")]
            remove_flags: Default::default(),

            #[cfg(feature = "message-add")]
            add_message: Default::default(),
            #[cfg(feature = "message-add")]
            add_message_with_flags: Default::default(),
            #[cfg(feature = "message-peek")]
            peek_messages: Default::default(),
            #[cfg(feature = "message-get")]
            get_messages: Default::default(),
            #[cfg(feature = "message-copy")]
            copy_messages: Default::default(),
            #[cfg(feature = "message-move")]
            move_messages: Default::default(),
            #[cfg(feature = "message-delete")]
            delete_messages: Default::default(),
            #[cfg(feature = "message-send")]
            send_message: Default::default(),
        }
    }

    #[cfg(feature = "folder-add")]
    pub fn set_add_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFolder>> + Send + Sync + 'static,
    ) {
        self.add_folder = Some(Arc::new(feature));
    }
    #[cfg(feature = "folder-add")]
    pub fn with_add_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_folder(feature);
        self
    }

    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) {
        self.list_folders = Some(Arc::new(feature));
    }
    #[cfg(feature = "folder-list")]
    pub fn with_list_folders(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_folders(feature);
        self
    }

    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ExpungeFolder>> + Send + Sync + 'static,
    ) {
        self.expunge_folder = Some(Arc::new(feature));
    }
    #[cfg(feature = "folder-expunge")]
    pub fn with_expunge_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ExpungeFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_expunge_folder(feature);
        self
    }

    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PurgeFolder>> + Send + Sync + 'static,
    ) {
        self.purge_folder = Some(Arc::new(feature));
    }
    #[cfg(feature = "folder-purge")]
    pub fn with_purge_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PurgeFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_purge_folder(feature);
        self
    }

    #[cfg(feature = "folder-delete")]
    pub fn set_delete_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteFolder>> + Send + Sync + 'static,
    ) {
        self.delete_folder = Some(Arc::new(feature));
    }
    #[cfg(feature = "folder-delete")]
    pub fn with_delete_folder(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_delete_folder(feature);
        self
    }

    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListEnvelopes>> + Send + Sync + 'static,
    ) {
        self.list_envelopes = Some(Arc::new(feature));
    }
    #[cfg(feature = "envelope-list")]
    pub fn with_list_envelopes(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn ListEnvelopes>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_envelopes(feature);
        self
    }

    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn WatchEnvelopes>> + Send + Sync + 'static,
    ) {
        self.watch_envelopes = Some(Arc::new(feature));
    }
    #[cfg(feature = "envelope-watch")]
    pub fn with_watch_envelopes(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn WatchEnvelopes>> + Send + Sync + 'static,
    ) -> Self {
        self.set_watch_envelopes(feature);
        self
    }

    #[cfg(feature = "envelope-get")]
    pub fn set_get_envelope(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetEnvelope>> + Send + Sync + 'static,
    ) {
        self.get_envelope = Some(Arc::new(feature));
    }
    #[cfg(feature = "envelope-get")]
    pub fn with_get_envelope(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetEnvelope>> + Send + Sync + 'static,
    ) -> Self {
        self.set_get_envelope(feature);
        self
    }

    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFlags>> + Send + Sync + 'static,
    ) {
        self.add_flags = Some(Arc::new(feature));
    }
    #[cfg(feature = "flag-add")]
    pub fn with_add_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_flags(feature);
        self
    }

    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SetFlags>> + Send + Sync + 'static,
    ) {
        self.set_flags = Some(Arc::new(feature));
    }
    #[cfg(feature = "flag-set")]
    pub fn with_set_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SetFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_set_flags(feature);
        self
    }

    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn RemoveFlags>> + Send + Sync + 'static,
    ) {
        self.remove_flags = Some(Arc::new(feature));
    }
    #[cfg(feature = "flag-remove")]
    pub fn with_remove_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn RemoveFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_remove_flags(feature);
        self
    }

    #[cfg(feature = "message-add")]
    pub fn set_add_message(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddMessage>> + Send + Sync + 'static,
    ) {
        self.add_message = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-add")]
    pub fn with_add_message(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddMessage>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_message(feature);
        self
    }

    #[cfg(feature = "message-add")]
    pub fn set_add_message_with_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddMessageWithFlags>> + Send + Sync + 'static,
    ) {
        self.add_message_with_flags = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-add")]
    pub fn with_add_message_with_flags(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn AddMessageWithFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_message_with_flags(feature);
        self
    }

    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PeekMessages>> + Send + Sync + 'static,
    ) {
        self.peek_messages = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-peek")]
    pub fn with_peek_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PeekMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_peek_messages(feature);
        self
    }

    #[cfg(feature = "message-get")]
    pub fn set_get_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetMessages>> + Send + Sync + 'static,
    ) {
        self.get_messages = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-get")]
    pub fn with_get_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn GetMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_get_messages(feature);
        self
    }

    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn CopyMessages>> + Send + Sync + 'static,
    ) {
        self.copy_messages = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-copy")]
    pub fn with_copy_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn CopyMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_copy_messages(feature);
        self
    }

    #[cfg(feature = "message-move")]
    pub fn set_move_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn MoveMessages>> + Send + Sync + 'static,
    ) {
        self.move_messages = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-move")]
    pub fn with_move_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn MoveMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_move_messages(feature);
        self
    }

    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteMessages>> + Send + Sync + 'static,
    ) {
        self.delete_messages = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-delete")]
    pub fn with_delete_messages(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_delete_messages(feature);
        self
    }

    #[cfg(feature = "message-send")]
    pub fn set_send_message(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SendMessage>> + Send + Sync + 'static,
    ) {
        self.send_message = Some(Arc::new(feature));
    }
    #[cfg(feature = "message-send")]
    pub fn with_send_message(
        mut self,
        feature: impl Fn(&C) -> Option<Box<dyn SendMessage>> + Send + Sync + 'static,
    ) -> Self {
        self.set_send_message(feature);
        self
    }

    pub async fn build(self) -> Result<Backend<C>> {
        let context = self.context_builder.build().await?;
        #[allow(unused_mut)]
        let mut backend = Backend::new(self.account_config.clone(), context);

        #[cfg(feature = "folder-add")]
        if let Some(feature) = &self.add_folder {
            backend.set_add_folder(feature(&backend.context));
        }
        #[cfg(feature = "folder-list")]
        if let Some(feature) = &self.list_folders {
            backend.set_list_folders(feature(&backend.context));
        }
        #[cfg(feature = "folder-expunge")]
        if let Some(feature) = &self.expunge_folder {
            backend.set_expunge_folder(feature(&backend.context));
        }
        #[cfg(feature = "folder-purge")]
        if let Some(feature) = &self.purge_folder {
            backend.set_purge_folder(feature(&backend.context));
        }
        #[cfg(feature = "folder-delete")]
        if let Some(feature) = &self.delete_folder {
            backend.set_delete_folder(feature(&backend.context));
        }

        #[cfg(feature = "envelope-list")]
        if let Some(feature) = &self.list_envelopes {
            backend.set_list_envelopes(feature(&backend.context));
        }
        #[cfg(feature = "envelope-watch")]
        if let Some(feature) = &self.watch_envelopes {
            backend.set_watch_envelopes(feature(&backend.context));
        }
        #[cfg(feature = "envelope-get")]
        if let Some(feature) = &self.get_envelope {
            backend.set_get_envelope(feature(&backend.context));
        }

        #[cfg(feature = "flag-add")]
        if let Some(feature) = &self.add_flags {
            backend.set_add_flags(feature(&backend.context));
        }
        #[cfg(feature = "flag-set")]
        if let Some(feature) = &self.set_flags {
            backend.set_set_flags(feature(&backend.context));
        }
        #[cfg(feature = "flag-remove")]
        if let Some(feature) = &self.remove_flags {
            backend.set_remove_flags(feature(&backend.context));
        }

        #[cfg(feature = "message-add")]
        if let Some(feature) = &self.add_message {
            backend.set_add_message(feature(&backend.context));
        }
        #[cfg(feature = "message-add")]
        if let Some(feature) = &self.add_message_with_flags {
            backend.set_add_message_with_flags(feature(&backend.context));
        }
        #[cfg(feature = "message-get")]
        if let Some(feature) = &self.get_messages {
            backend.set_get_messages(feature(&backend.context));
        }
        #[cfg(feature = "message-peek")]
        if let Some(feature) = &self.peek_messages {
            backend.set_peek_messages(feature(&backend.context));
        }
        #[cfg(feature = "message-copy")]
        if let Some(feature) = &self.copy_messages {
            backend.set_copy_messages(feature(&backend.context));
        }
        #[cfg(feature = "message-move")]
        if let Some(feature) = &self.move_messages {
            backend.set_move_messages(feature(&backend.context));
        }
        #[cfg(feature = "message-delete")]
        if let Some(feature) = &self.delete_messages {
            backend.set_delete_messages(feature(&backend.context));
        }
        #[cfg(feature = "message-send")]
        if let Some(feature) = self.send_message {
            backend.set_send_message(feature(&backend.context));
        }

        Ok(backend)
    }
}

impl<B: BackendContextBuilder> Clone for BackendBuilder<B> {
    fn clone(&self) -> Self {
        Self {
            context_builder: self.context_builder.clone(),

            account_config: self.account_config.clone(),

            #[cfg(feature = "folder-add")]
            add_folder: self.add_folder.clone(),
            #[cfg(feature = "folder-list")]
            list_folders: self.list_folders.clone(),
            #[cfg(feature = "folder-expunge")]
            expunge_folder: self.expunge_folder.clone(),
            #[cfg(feature = "folder-purge")]
            purge_folder: self.purge_folder.clone(),
            #[cfg(feature = "folder-delete")]
            delete_folder: self.delete_folder.clone(),

            #[cfg(feature = "envelope-list")]
            list_envelopes: self.list_envelopes.clone(),
            #[cfg(feature = "envelope-watch")]
            watch_envelopes: self.watch_envelopes.clone(),
            #[cfg(feature = "envelope-get")]
            get_envelope: self.get_envelope.clone(),

            #[cfg(feature = "flag-add")]
            add_flags: self.add_flags.clone(),
            #[cfg(feature = "flag-set")]
            set_flags: self.set_flags.clone(),
            #[cfg(feature = "flag-remove")]
            remove_flags: self.remove_flags.clone(),

            #[cfg(feature = "message-add")]
            add_message: self.add_message.clone(),
            #[cfg(feature = "message-add")]
            add_message_with_flags: self.add_message_with_flags.clone(),
            #[cfg(feature = "message-peek")]
            peek_messages: self.peek_messages.clone(),
            #[cfg(feature = "message-get")]
            get_messages: self.get_messages.clone(),
            #[cfg(feature = "message-copy")]
            copy_messages: self.copy_messages.clone(),
            #[cfg(feature = "message-move")]
            move_messages: self.move_messages.clone(),
            #[cfg(feature = "message-delete")]
            delete_messages: self.delete_messages.clone(),
            #[cfg(feature = "message-send")]
            send_message: self.send_message.clone(),
        }
    }
}

impl Default for BackendBuilder<()> {
    fn default() -> Self {
        Self {
            context_builder: (),

            account_config: Default::default(),

            #[cfg(feature = "folder-add")]
            add_folder: Default::default(),
            #[cfg(feature = "folder-list")]
            list_folders: Default::default(),
            #[cfg(feature = "folder-expunge")]
            expunge_folder: Default::default(),
            #[cfg(feature = "folder-purge")]
            purge_folder: Default::default(),
            #[cfg(feature = "folder-delete")]
            delete_folder: Default::default(),

            #[cfg(feature = "envelope-list")]
            list_envelopes: Default::default(),
            #[cfg(feature = "envelope-watch")]
            watch_envelopes: Default::default(),
            #[cfg(feature = "envelope-get")]
            get_envelope: Default::default(),

            #[cfg(feature = "flag-add")]
            add_flags: Default::default(),
            #[cfg(feature = "flag-set")]
            set_flags: Default::default(),
            #[cfg(feature = "flag-remove")]
            remove_flags: Default::default(),

            #[cfg(feature = "message-add")]
            add_message: Default::default(),
            #[cfg(feature = "message-add")]
            add_message_with_flags: Default::default(),
            #[cfg(feature = "message-peek")]
            peek_messages: Default::default(),
            #[cfg(feature = "message-get")]
            get_messages: Default::default(),
            #[cfg(feature = "message-copy")]
            copy_messages: Default::default(),
            #[cfg(feature = "message-move")]
            move_messages: Default::default(),
            #[cfg(feature = "message-delete")]
            delete_messages: Default::default(),
            #[cfg(feature = "message-send")]
            send_message: Default::default(),
        }
    }
}

pub struct Backend<C> {
    #[allow(dead_code)]
    context: C,

    pub account_config: AccountConfig,

    #[cfg(feature = "folder-add")]
    pub add_folder: Option<Box<dyn AddFolder>>,
    #[cfg(feature = "folder-list")]
    pub list_folders: Option<Box<dyn ListFolders>>,
    #[cfg(feature = "folder-expunge")]
    pub expunge_folder: Option<Box<dyn ExpungeFolder>>,
    #[cfg(feature = "folder-purge")]
    pub purge_folder: Option<Box<dyn PurgeFolder>>,
    #[cfg(feature = "folder-delete")]
    pub delete_folder: Option<Box<dyn DeleteFolder>>,

    #[cfg(feature = "envelope-list")]
    pub list_envelopes: Option<Box<dyn ListEnvelopes>>,
    #[cfg(feature = "envelope-watch")]
    pub watch_envelopes: Option<Box<dyn WatchEnvelopes>>,
    #[cfg(feature = "envelope-get")]
    pub get_envelope: Option<Box<dyn GetEnvelope>>,

    #[cfg(feature = "flag-add")]
    pub add_flags: Option<Box<dyn AddFlags>>,
    #[cfg(feature = "flag-set")]
    pub set_flags: Option<Box<dyn SetFlags>>,
    #[cfg(feature = "flag-remove")]
    pub remove_flags: Option<Box<dyn RemoveFlags>>,

    #[cfg(feature = "message-add")]
    pub add_message: Option<Box<dyn AddMessage>>,
    #[cfg(feature = "message-add")]
    pub add_message_with_flags: Option<Box<dyn AddMessageWithFlags>>,
    #[cfg(feature = "message-peek")]
    pub peek_messages: Option<Box<dyn PeekMessages>>,
    #[cfg(feature = "message-get")]
    pub get_messages: Option<Box<dyn GetMessages>>,
    #[cfg(feature = "message-copy")]
    pub copy_messages: Option<Box<dyn CopyMessages>>,
    #[cfg(feature = "message-move")]
    pub move_messages: Option<Box<dyn MoveMessages>>,
    #[cfg(feature = "message-delete")]
    pub delete_messages: Option<Box<dyn DeleteMessages>>,
    #[cfg(feature = "message-send")]
    pub send_message: Option<Box<dyn SendMessage>>,
}

impl<C> Backend<C> {
    pub fn new(account_config: AccountConfig, context: C) -> Backend<C> {
        Backend {
            context,

            account_config,

            #[cfg(feature = "folder-add")]
            add_folder: None,
            #[cfg(feature = "folder-list")]
            list_folders: None,
            #[cfg(feature = "folder-expunge")]
            expunge_folder: None,
            #[cfg(feature = "folder-purge")]
            purge_folder: None,
            #[cfg(feature = "folder-delete")]
            delete_folder: None,

            #[cfg(feature = "envelope-list")]
            list_envelopes: None,
            #[cfg(feature = "envelope-watch")]
            watch_envelopes: None,
            #[cfg(feature = "envelope-get")]
            get_envelope: None,

            #[cfg(feature = "flag-add")]
            add_flags: None,
            #[cfg(feature = "flag-set")]
            set_flags: None,
            #[cfg(feature = "flag-remove")]
            remove_flags: None,

            #[cfg(feature = "message-add")]
            add_message: None,
            #[cfg(feature = "message-add")]
            add_message_with_flags: None,
            #[cfg(feature = "message-peek")]
            peek_messages: None,
            #[cfg(feature = "message-get")]
            get_messages: None,
            #[cfg(feature = "message-copy")]
            copy_messages: None,
            #[cfg(feature = "message-move")]
            move_messages: None,
            #[cfg(feature = "message-delete")]
            delete_messages: None,
            #[cfg(feature = "message-send")]
            send_message: None,
        }
    }

    #[cfg(feature = "folder-add")]
    pub fn set_add_folder(&mut self, feature: Option<Box<dyn AddFolder>>) {
        self.add_folder = feature;
    }
    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(&mut self, feature: Option<Box<dyn ListFolders>>) {
        self.list_folders = feature;
    }
    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(&mut self, feature: Option<Box<dyn ExpungeFolder>>) {
        self.expunge_folder = feature;
    }
    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(&mut self, feature: Option<Box<dyn PurgeFolder>>) {
        self.purge_folder = feature;
    }
    #[cfg(feature = "folder-delete")]
    pub fn set_delete_folder(&mut self, feature: Option<Box<dyn DeleteFolder>>) {
        self.delete_folder = feature;
    }

    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(&mut self, feature: Option<Box<dyn ListEnvelopes>>) {
        self.list_envelopes = feature;
    }
    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(&mut self, feature: Option<Box<dyn WatchEnvelopes>>) {
        self.watch_envelopes = feature;
    }
    #[cfg(feature = "envelope-get")]
    pub fn set_get_envelope(&mut self, feature: Option<Box<dyn GetEnvelope>>) {
        self.get_envelope = feature;
    }

    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(&mut self, feature: Option<Box<dyn AddFlags>>) {
        self.add_flags = feature;
    }
    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(&mut self, feature: Option<Box<dyn SetFlags>>) {
        self.set_flags = feature;
    }
    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(&mut self, feature: Option<Box<dyn RemoveFlags>>) {
        self.remove_flags = feature;
    }

    #[cfg(feature = "message-add")]
    pub fn set_add_message(&mut self, feature: Option<Box<dyn AddMessage>>) {
        self.add_message = feature;
    }
    #[cfg(feature = "message-add")]
    pub fn set_add_message_with_flags(&mut self, feature: Option<Box<dyn AddMessageWithFlags>>) {
        self.add_message_with_flags = feature;
    }
    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(&mut self, feature: Option<Box<dyn PeekMessages>>) {
        self.peek_messages = feature;
    }
    #[cfg(feature = "message-get")]
    pub fn set_get_messages(&mut self, feature: Option<Box<dyn GetMessages>>) {
        self.get_messages = feature;
    }
    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(&mut self, feature: Option<Box<dyn CopyMessages>>) {
        self.copy_messages = feature;
    }
    #[cfg(feature = "message-move")]
    pub fn set_move_messages(&mut self, feature: Option<Box<dyn MoveMessages>>) {
        self.move_messages = feature;
    }
    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(&mut self, feature: Option<Box<dyn DeleteMessages>>) {
        self.delete_messages = feature;
    }
    #[cfg(feature = "message-send")]
    pub fn set_send_message(&mut self, feature: Option<Box<dyn SendMessage>>) {
        self.send_message = feature;
    }

    #[cfg(feature = "folder-add")]
    pub async fn add_folder(&self, folder: &str) -> Result<()> {
        self.add_folder
            .as_ref()
            .ok_or(Error::AddFolderNotAvailableError)?
            .add_folder(folder)
            .await
    }

    #[cfg(feature = "folder-list")]
    pub async fn list_folders(&self) -> Result<Folders> {
        self.list_folders
            .as_ref()
            .ok_or(Error::ListFoldersNotAvailableError)?
            .list_folders()
            .await
    }

    #[cfg(feature = "folder-expunge")]
    pub async fn expunge_folder(&self, folder: &str) -> Result<()> {
        self.expunge_folder
            .as_ref()
            .ok_or(Error::ExpungeFolderNotAvailableError)?
            .expunge_folder(folder)
            .await
    }

    #[cfg(feature = "folder-purge")]
    pub async fn purge_folder(&self, folder: &str) -> Result<()> {
        self.purge_folder
            .as_ref()
            .ok_or(Error::PurgeFolderNotAvailableError)?
            .purge_folder(folder)
            .await
    }

    #[cfg(feature = "folder-delete")]
    pub async fn delete_folder(&self, folder: &str) -> Result<()> {
        self.delete_folder
            .as_ref()
            .ok_or(Error::DeleteFolderNotAvailableError)?
            .delete_folder(folder)
            .await
    }

    #[cfg(feature = "envelope-list")]
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

    #[cfg(feature = "envelope-watch")]
    pub async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        self.watch_envelopes
            .as_ref()
            .ok_or(Error::WatchEnvelopesNotAvailableError)?
            .watch_envelopes(folder)
            .await
    }

    #[cfg(feature = "envelope-get")]
    pub async fn get_envelope(&self, folder: &str, id: &Id) -> Result<Envelope> {
        self.get_envelope
            .as_ref()
            .ok_or(Error::GetEnvelopeNotAvailableError)?
            .get_envelope(folder, id)
            .await
    }

    #[cfg(feature = "flag-add")]
    pub async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.add_flags
            .as_ref()
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flags(folder, id, flags)
            .await
    }

    #[cfg(feature = "flag-add")]
    pub async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.add_flags
            .as_ref()
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flag(folder, id, flag)
            .await
    }

    #[cfg(feature = "flag-set")]
    pub async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.set_flags
            .as_ref()
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flags(folder, id, flags)
            .await
    }

    #[cfg(feature = "flag-set")]
    pub async fn set_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.set_flags
            .as_ref()
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flag(folder, id, flag)
            .await
    }

    #[cfg(feature = "flag-remove")]
    pub async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flags(folder, id, flags)
            .await
    }

    #[cfg(feature = "flag-remove")]
    pub async fn remove_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flag(folder, id, flag)
            .await
    }

    #[cfg(feature = "message-add")]
    pub async fn add_message(&self, folder: &str, raw_msg: &[u8]) -> Result<SingleId> {
        if let Some(f) = self.add_message.as_ref() {
            return f.add_message(folder, raw_msg).await;
        }

        #[cfg(feature = "flag-add")]
        if let Some(f) = self.add_message_with_flags.as_ref() {
            return f
                .add_message_with_flags(folder, raw_msg, &Default::default())
                .await;
        }

        Err(Error::AddMessageNotAvailableError.into())
    }

    #[cfg(feature = "message-add")]
    pub async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        if let Some(f) = self.add_message_with_flags.as_ref() {
            return f.add_message_with_flags(folder, raw_msg, flags).await;
        }

        #[cfg(feature = "flag-add")]
        if let (Some(a), Some(b)) = (self.add_message.as_ref(), self.add_flags.as_ref()) {
            return default_add_message_with_flags(a.as_ref(), b.as_ref(), folder, raw_msg, flags)
                .await;
        }

        Err(Error::AddMessageWithFlagsNotAvailableError.into())
    }

    #[cfg(feature = "message-add")]
    pub async fn add_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<SingleId> {
        if let Some(f) = self.add_message_with_flags.as_ref() {
            return f.add_message_with_flag(folder, raw_msg, flag).await;
        }

        #[cfg(feature = "flag-add")]
        if let (Some(a), Some(b)) = (self.add_message.as_ref(), self.add_flags.as_ref()) {
            return default_add_message_with_flag(a.as_ref(), b.as_ref(), folder, raw_msg, flag)
                .await;
        }

        Err(Error::AddMessageWithFlagsNotAvailableError.into())
    }

    #[cfg(feature = "message-peek")]
    pub async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        self.peek_messages
            .as_ref()
            .ok_or(Error::PeekMessagesNotAvailableError)?
            .peek_messages(folder, id)
            .await
    }

    #[cfg(feature = "message-get")]
    pub async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        if let Some(f) = self.get_messages.as_ref() {
            return f.get_messages(folder, id).await;
        }

        #[cfg(all(feature = "message-peek", feature = "flag-add"))]
        if let (Some(a), Some(b)) = (self.peek_messages.as_ref(), self.add_flags.as_ref()) {
            return default_get_messages(a.as_ref(), b.as_ref(), folder, id).await;
        }

        Err(Error::PeekMessagesNotAvailableError.into())
    }

    #[cfg(feature = "message-copy")]
    pub async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.copy_messages
            .as_ref()
            .ok_or(Error::CopyMessagesNotAvailableError)?
            .copy_messages(from_folder, to_folder, id)
            .await
    }

    #[cfg(feature = "message-move")]
    pub async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.move_messages
            .as_ref()
            .ok_or(Error::MoveMessagesNotAvailableError)?
            .move_messages(from_folder, to_folder, id)
            .await
    }

    #[cfg(feature = "message-delete")]
    pub async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        if let Some(f) = self.delete_messages.as_ref() {
            return f.delete_messages(folder, id).await;
        }

        #[cfg(all(feature = "message-move", feature = "flag-add"))]
        if let (Some(a), Some(b)) = (self.move_messages.as_ref(), self.add_flags.as_ref()) {
            return default_delete_messages(
                &self.account_config,
                a.as_ref(),
                b.as_ref(),
                folder,
                id,
            )
            .await;
        }

        Err(Error::DeleteMessagesNotAvailableError.into())
    }

    #[cfg(feature = "message-send")]
    pub async fn send_message(&self, raw_msg: &[u8]) -> Result<()> {
        self.send_message
            .as_ref()
            .ok_or(Error::SendMessageNotAvailableError)?
            .send_message(raw_msg)
            .await?;

        #[cfg(feature = "message-add")]
        if self.account_config.should_save_copy_sent_message() {
            let folder = self.account_config.get_sent_folder_alias();
            log::debug!("saving copy of sent message to {folder}");
            self.add_message_with_flag(&folder, raw_msg, Flag::Seen)
                .await?;
        }

        Ok(())
    }
}
