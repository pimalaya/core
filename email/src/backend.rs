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
use tokio::sync::Mutex;

#[cfg(feature = "envelope-get")]
use crate::envelope::get::GetEnvelope;
#[cfg(feature = "envelope-list")]
use crate::envelope::list::ListEnvelopes;
#[cfg(feature = "envelope-watch")]
use crate::envelope::watch::WatchEnvelopes;
#[cfg(feature = "flag-add")]
use crate::flag::add::AddFlags;
#[cfg(feature = "flag-remove")]
use crate::flag::remove::RemoveFlags;
#[cfg(feature = "flag-set")]
use crate::flag::set::SetFlags;
#[cfg(feature = "folder-add")]
use crate::folder::add::AddFolder;
#[cfg(feature = "folder-delete")]
use crate::folder::delete::DeleteFolder;
#[cfg(feature = "folder-expunge")]
use crate::folder::expunge::ExpungeFolder;
#[cfg(feature = "folder-list")]
use crate::folder::list::ListFolders;
#[cfg(feature = "folder-purge")]
use crate::folder::purge::PurgeFolder;
#[cfg(feature = "message-add")]
use crate::message::add::AddMessage;
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
#[allow(unused)]
use crate::{
    account::config::AccountConfig,
    envelope::{Envelope, Envelopes},
    envelope::{Id, SingleId},
    flag::{Flag, Flags},
    folder::Folders,
    message::Messages,
    Result,
};

/// Errors related to backend.
#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "folder-add")]
    #[error("cannot add folder: feature not available")]
    AddFolderNotAvailableError,
    #[cfg(feature = "folder-list")]
    #[error("cannot list folders: feature not available")]
    ListFoldersNotAvailableError,
    #[cfg(feature = "folder-expunge")]
    #[error("cannot expunge folder: feature not available")]
    ExpungeFolderNotAvailableError,
    #[cfg(feature = "folder-purge")]
    #[error("cannot purge folder: feature not available")]
    PurgeFolderNotAvailableError,
    #[cfg(feature = "folder-delete")]
    #[error("cannot delete folder: feature not available")]
    DeleteFolderNotAvailableError,

    #[cfg(feature = "envelope-list")]
    #[error("cannot list envelopes: feature not available")]
    ListEnvelopesNotAvailableError,
    #[cfg(feature = "envelope-watch")]
    #[error("cannot watch for envelopes changes: feature not available")]
    WatchEnvelopesNotAvailableError,
    #[cfg(feature = "envelope-get")]
    #[error("cannot get envelope: feature not available")]
    GetEnvelopeNotAvailableError,

    #[cfg(feature = "flag-add")]
    #[error("cannot add flag(s): feature not available")]
    AddFlagsNotAvailableError,
    #[cfg(feature = "flag-set")]
    #[error("cannot set flag(s): feature not available")]
    SetFlagsNotAvailableError,
    #[cfg(feature = "flag-remove")]
    #[error("cannot remove flag(s): feature not available")]
    RemoveFlagsNotAvailableError,

    #[cfg(feature = "message-add")]
    #[error("cannot add message: feature not available")]
    AddMessageNotAvailableError,
    #[cfg(feature = "message-add")]
    #[error("cannot add message with flags: feature not available")]
    AddMessageWithFlagsNotAvailableError,
    #[cfg(feature = "message-get")]
    #[error("cannot get messages: feature not available")]
    GetMessagesNotAvailableError,
    #[cfg(feature = "message-peek")]
    #[error("cannot peek messages: feature not available")]
    PeekMessagesNotAvailableError,
    #[cfg(feature = "message-copy")]
    #[error("cannot copy messages: feature not available")]
    CopyMessagesNotAvailableError,
    #[cfg(feature = "message-move")]
    #[error("cannot move messages: feature not available")]
    MoveMessagesNotAvailableError,
    #[cfg(feature = "message-delete")]
    #[error("cannot delete messages: feature not available")]
    DeleteMessagesNotAvailableError,
    #[cfg(feature = "message-send")]
    #[error("cannot send message: feature not available")]
    SendMessageNotAvailableError,
}

#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
    type Context: Send;

    async fn build(self, account_config: &AccountConfig) -> Result<Self::Context>;
}

#[async_trait]
impl BackendContextBuilder for () {
    type Context = ();

    async fn build(self, _account_config: &AccountConfig) -> Result<Self::Context> {
        Ok(())
    }
}

#[async_trait]
impl<T: BackendContextBuilder, U: BackendContextBuilder> BackendContextBuilder for (T, U) {
    type Context = (T::Context, U::Context);

    async fn build(self, account_config: &AccountConfig) -> Result<Self::Context> {
        Ok((
            self.0.build(account_config).await?,
            self.1.build(account_config).await?,
        ))
    }
}

pub struct BackendBuilder<B: BackendContextBuilder> {
    pub account_config: AccountConfig,
    pub context_builder: B,

    #[cfg(feature = "folder-add")]
    add_folder: Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddFolder>> + Send + Sync>,

    #[cfg(feature = "folder-list")]
    list_folders: Arc<dyn Fn(&B::Context) -> Option<Box<dyn ListFolders>> + Send + Sync>,

    #[cfg(feature = "folder-expunge")]
    expunge_folder: Arc<dyn Fn(&B::Context) -> Option<Box<dyn ExpungeFolder>> + Send + Sync>,

    #[cfg(feature = "folder-purge")]
    purge_folder: Arc<dyn Fn(&B::Context) -> Option<Box<dyn PurgeFolder>> + Send + Sync>,

    #[cfg(feature = "folder-delete")]
    delete_folder: Arc<dyn Fn(&B::Context) -> Option<Box<dyn DeleteFolder>> + Send + Sync>,

    #[cfg(feature = "envelope-list")]
    list_envelopes: Arc<dyn Fn(&B::Context) -> Option<Box<dyn ListEnvelopes>> + Send + Sync>,

    #[cfg(feature = "envelope-watch")]
    watch_envelopes: Arc<dyn Fn(&B::Context) -> Option<Box<dyn WatchEnvelopes>> + Send + Sync>,

    #[cfg(feature = "envelope-get")]
    get_envelope: Arc<dyn Fn(&B::Context) -> Option<Box<dyn GetEnvelope>> + Send + Sync>,

    #[cfg(feature = "flag-add")]
    add_flags: Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddFlags>> + Send + Sync>,

    #[cfg(feature = "flag-set")]
    set_flags: Arc<dyn Fn(&B::Context) -> Option<Box<dyn SetFlags>> + Send + Sync>,

    #[cfg(feature = "flag-remove")]
    remove_flags: Arc<dyn Fn(&B::Context) -> Option<Box<dyn RemoveFlags>> + Send + Sync>,

    #[cfg(feature = "message-add")]
    add_message: Arc<dyn Fn(&B::Context) -> Option<Box<dyn AddMessage>> + Send + Sync>,

    #[cfg(feature = "message-peek")]
    peek_messages: Arc<dyn Fn(&B::Context) -> Option<Box<dyn PeekMessages>> + Send + Sync>,

    #[cfg(feature = "message-get")]
    get_messages: Arc<dyn Fn(&B::Context) -> Option<Box<dyn GetMessages>> + Send + Sync>,

    #[cfg(feature = "message-copy")]
    copy_messages: Arc<dyn Fn(&B::Context) -> Option<Box<dyn CopyMessages>> + Send + Sync>,

    #[cfg(feature = "message-move")]
    move_messages: Arc<dyn Fn(&B::Context) -> Option<Box<dyn MoveMessages>> + Send + Sync>,

    #[cfg(feature = "message-delete")]
    delete_messages: Arc<dyn Fn(&B::Context) -> Option<Box<dyn DeleteMessages>> + Send + Sync>,

    #[cfg(feature = "message-send")]
    send_message: Arc<dyn Fn(&B::Context) -> Option<Box<dyn SendMessage>> + Send + Sync>,
}

impl<C: Send, B: BackendContextBuilder<Context = C>> BackendBuilder<B> {
    pub fn new(account_config: AccountConfig, context_builder: B) -> Self {
        Self {
            account_config,
            context_builder,

            #[cfg(feature = "folder-add")]
            add_folder: Arc::new(|_| None),

            #[cfg(feature = "folder-list")]
            list_folders: Arc::new(|_| None),
            #[cfg(feature = "folder-expunge")]
            expunge_folder: Arc::new(|_| None),
            #[cfg(feature = "folder-purge")]
            purge_folder: Arc::new(|_| None),
            #[cfg(feature = "folder-delete")]
            delete_folder: Arc::new(|_| None),

            #[cfg(feature = "envelope-list")]
            list_envelopes: Arc::new(|_| None),
            #[cfg(feature = "envelope-watch")]
            watch_envelopes: Arc::new(|_| None),
            #[cfg(feature = "envelope-get")]
            get_envelope: Arc::new(|_| None),

            #[cfg(feature = "flag-add")]
            add_flags: Arc::new(|_| None),
            #[cfg(feature = "flag-set")]
            set_flags: Arc::new(|_| None),
            #[cfg(feature = "flag-remove")]
            remove_flags: Arc::new(|_| None),

            #[cfg(feature = "message-add")]
            add_message: Arc::new(|_| None),
            #[cfg(feature = "message-peek")]
            peek_messages: Arc::new(|_| None),
            #[cfg(feature = "message-get")]
            get_messages: Arc::new(|_| None),
            #[cfg(feature = "message-copy")]
            copy_messages: Arc::new(|_| None),
            #[cfg(feature = "message-move")]
            move_messages: Arc::new(|_| None),
            #[cfg(feature = "message-delete")]
            delete_messages: Arc::new(|_| None),
            #[cfg(feature = "message-send")]
            send_message: Arc::new(|_| None),
        }
    }

    #[cfg(feature = "folder-add")]
    pub fn set_add_folder(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn AddFolder>> + Send + Sync + 'static,
    ) {
        self.add_folder = Arc::new(f);
    }

    #[cfg(feature = "folder-add")]
    pub fn with_add_folder(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn AddFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_folder(f);
        self
    }

    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) {
        self.list_folders = Arc::new(f);
    }

    #[cfg(feature = "folder-list")]
    pub fn with_list_folders(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_folders(f);
        self
    }

    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn ExpungeFolder>> + Send + Sync + 'static,
    ) {
        self.expunge_folder = Arc::new(f);
    }

    #[cfg(feature = "folder-expunge")]
    pub fn with_expunge_folder(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn ExpungeFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_expunge_folder(f);
        self
    }

    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn PurgeFolder>> + Send + Sync + 'static,
    ) {
        self.purge_folder = Arc::new(feature);
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
        f: impl Fn(&C) -> Option<Box<dyn DeleteFolder>> + Send + Sync + 'static,
    ) {
        self.delete_folder = Arc::new(f);
    }

    #[cfg(feature = "folder-delete")]
    pub fn with_delete_folder(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn DeleteFolder>> + Send + Sync + 'static,
    ) -> Self {
        self.set_delete_folder(f);
        self
    }

    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn ListEnvelopes>> + Send + Sync + 'static,
    ) {
        self.list_envelopes = Arc::new(f);
    }

    #[cfg(feature = "envelope-list")]
    pub fn with_list_envelopes(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn ListEnvelopes>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_envelopes(f);
        self
    }

    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn WatchEnvelopes>> + Send + Sync + 'static,
    ) {
        self.watch_envelopes = Arc::new(feature);
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
        f: impl Fn(&C) -> Option<Box<dyn GetEnvelope>> + Send + Sync + 'static,
    ) {
        self.get_envelope = Arc::new(f);
    }

    #[cfg(feature = "envelope-get")]
    pub fn with_get_envelope(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn GetEnvelope>> + Send + Sync + 'static,
    ) -> Self {
        self.set_get_envelope(f);
        self
    }

    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn AddFlags>> + Send + Sync + 'static,
    ) {
        self.add_flags = Arc::new(f);
    }

    #[cfg(feature = "flag-add")]
    pub fn with_add_flags(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn AddFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_flags(f);
        self
    }

    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn SetFlags>> + Send + Sync + 'static,
    ) {
        self.set_flags = Arc::new(f);
    }

    #[cfg(feature = "flag-set")]
    pub fn with_set_flags(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn SetFlags>> + Send + Sync + 'static,
    ) -> Self {
        self.set_set_flags(f);
        self
    }

    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn RemoveFlags>> + Send + Sync + 'static,
    ) {
        self.remove_flags = Arc::new(feature);
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
        f: impl Fn(&C) -> Option<Box<dyn AddMessage>> + Send + Sync + 'static,
    ) {
        self.add_message = Arc::new(f);
    }

    #[cfg(feature = "message-add")]
    pub fn with_add_message(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn AddMessage>> + Send + Sync + 'static,
    ) -> Self {
        self.set_add_message(f);
        self
    }

    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn PeekMessages>> + Send + Sync + 'static,
    ) {
        self.peek_messages = Arc::new(f);
    }

    #[cfg(feature = "message-peek")]
    pub fn with_peek_messages(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn PeekMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_peek_messages(f);
        self
    }

    #[cfg(feature = "message-get")]
    pub fn set_get_messages(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn GetMessages>> + Send + Sync + 'static,
    ) {
        self.get_messages = Arc::new(f);
    }

    #[cfg(feature = "message-get")]
    pub fn with_get_messages(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn GetMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_get_messages(f);
        self
    }

    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn CopyMessages>> + Send + Sync + 'static,
    ) {
        self.copy_messages = Arc::new(feature);
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
        f: impl Fn(&C) -> Option<Box<dyn MoveMessages>> + Send + Sync + 'static,
    ) {
        self.move_messages = Arc::new(f);
    }

    #[cfg(feature = "message-move")]
    pub fn with_move_messages(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn MoveMessages>> + Send + Sync + 'static,
    ) -> Self {
        self.set_move_messages(f);
        self
    }

    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(
        &mut self,
        feature: impl Fn(&C) -> Option<Box<dyn DeleteMessages>> + Send + Sync + 'static,
    ) {
        self.delete_messages = Arc::new(feature);
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
        self.send_message = Arc::new(feature);
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
        let context = self.context_builder.build(&self.account_config).await?;

        #[allow(unused_mut)]
        let mut backend = Backend::new(self.account_config);

        #[cfg(feature = "folder-add")]
        backend.set_add_folder((self.add_folder)(&context));

        #[cfg(feature = "folder-list")]
        backend.set_list_folders((self.list_folders)(&context));

        #[cfg(feature = "folder-expunge")]
        backend.set_expunge_folder((self.expunge_folder)(&context));

        #[cfg(feature = "folder-purge")]
        backend.set_purge_folder((self.purge_folder)(&context));

        #[cfg(feature = "folder-delete")]
        backend.set_delete_folder((self.delete_folder)(&context));

        #[cfg(feature = "envelope-list")]
        backend.set_list_envelopes((self.list_envelopes)(&context));

        #[cfg(feature = "envelope-watch")]
        backend.set_watch_envelopes((self.watch_envelopes)(&context));

        #[cfg(feature = "envelope-get")]
        backend.set_get_envelope((self.get_envelope)(&context));

        #[cfg(feature = "flag-add")]
        backend.set_add_flags((self.add_flags)(&context));

        #[cfg(feature = "flag-set")]
        backend.set_set_flags((self.set_flags)(&context));

        #[cfg(feature = "flag-remove")]
        backend.set_remove_flags((self.remove_flags)(&context));

        #[cfg(feature = "message-add")]
        backend.set_add_message((self.add_message)(&context));

        #[cfg(feature = "message-get")]
        backend.set_get_messages((self.get_messages)(&context));

        #[cfg(feature = "message-peek")]
        backend.set_peek_messages((self.peek_messages)(&context));

        #[cfg(feature = "message-copy")]
        backend.set_copy_messages((self.copy_messages)(&context));

        #[cfg(feature = "message-move")]
        backend.set_move_messages((self.move_messages)(&context));

        #[cfg(feature = "message-delete")]
        backend.set_delete_messages((self.delete_messages)(&context));

        #[cfg(feature = "message-send")]
        backend.set_send_message((self.send_message)(&context));

        backend.set_context(context);

        Ok(backend)
    }
}

impl<B: BackendContextBuilder> Clone for BackendBuilder<B> {
    fn clone(&self) -> Self {
        Self {
            account_config: self.account_config.clone(),
            context_builder: self.context_builder.clone(),

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
            add_folder: Arc::new(|_| None),

            #[cfg(feature = "folder-list")]
            list_folders: Arc::new(|_| None),

            #[cfg(feature = "folder-expunge")]
            expunge_folder: Arc::new(|_| None),

            #[cfg(feature = "folder-purge")]
            purge_folder: Arc::new(|_| None),

            #[cfg(feature = "folder-delete")]
            delete_folder: Arc::new(|_| None),

            #[cfg(feature = "envelope-list")]
            list_envelopes: Arc::new(|_| None),

            #[cfg(feature = "envelope-watch")]
            watch_envelopes: Arc::new(|_| None),

            #[cfg(feature = "envelope-get")]
            get_envelope: Arc::new(|_| None),

            #[cfg(feature = "flag-add")]
            add_flags: Arc::new(|_| None),

            #[cfg(feature = "flag-set")]
            set_flags: Arc::new(|_| None),

            #[cfg(feature = "flag-remove")]
            remove_flags: Arc::new(|_| None),

            #[cfg(feature = "message-add")]
            add_message: Arc::new(|_| None),

            #[cfg(feature = "message-peek")]
            peek_messages: Arc::new(|_| None),

            #[cfg(feature = "message-get")]
            get_messages: Arc::new(|_| None),

            #[cfg(feature = "message-copy")]
            copy_messages: Arc::new(|_| None),

            #[cfg(feature = "message-move")]
            move_messages: Arc::new(|_| None),

            #[cfg(feature = "message-delete")]
            delete_messages: Arc::new(|_| None),

            #[cfg(feature = "message-send")]
            send_message: Arc::new(|_| None),
        }
    }
}

pub struct Backend<C: Send> {
    pub account_config: AccountConfig,
    #[allow(dead_code)]
    pub context: Option<Mutex<C>>,

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

impl<C: Send> Backend<C> {
    pub fn new(account_config: AccountConfig) -> Backend<C> {
        Backend {
            account_config,
            context: None,

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
    pub async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        self.add_message
            .as_ref()
            .ok_or(Error::AddMessageWithFlagsNotAvailableError)?
            .add_message_with_flags(folder, raw_msg, flags)
            .await
    }

    #[cfg(feature = "message-add")]
    pub async fn add_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<SingleId> {
        self.add_message
            .as_ref()
            .ok_or(Error::AddMessageWithFlagsNotAvailableError)?
            .add_message_with_flag(folder, raw_msg, flag)
            .await
    }

    #[cfg(feature = "message-add")]
    pub async fn add_message(&self, folder: &str, raw_msg: &[u8]) -> Result<SingleId> {
        self.add_message
            .as_ref()
            .ok_or(Error::AddMessageWithFlagsNotAvailableError)?
            .add_message(folder, raw_msg)
            .await
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

    fn set_context(&mut self, context: C) {
        self.context = Some(Mutex::new(context));
    }
}

#[async_trait]
pub trait BackendContextBuilderV2: Clone + Send + Sync {
    type Context: Send;

    #[cfg(feature = "folder-list")]
    fn list_folders_builder(
        &self,
    ) -> Option<Arc<dyn Fn(&Self::Context) -> Option<Box<dyn ListFolders>> + Send + Sync>> {
        None
    }

    async fn build(self, account_config: &AccountConfig) -> Result<Self::Context>;
}

pub struct BackendBuilderV2<CB: BackendContextBuilderV2> {
    context_builder: CB,

    #[cfg(feature = "folder-list")]
    list_folders_builder:
        Option<Arc<dyn Fn(&CB::Context) -> Option<Box<dyn ListFolders>> + Send + Sync>>,
}

impl<C: Send, CB: BackendContextBuilderV2<Context = C>> BackendBuilderV2<CB> {
    pub fn new(context_builder: CB) -> Self {
        Self {
            context_builder,
            list_folders_builder: None,
        }
    }

    #[cfg(feature = "folder-list")]
    pub fn set_list_folders_builder(
        &mut self,
        f: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) {
        self.list_folders_builder = Some(Arc::new(f));
    }

    #[cfg(feature = "folder-list")]
    pub fn with_list_folders_builder(
        mut self,
        f: impl Fn(&C) -> Option<Box<dyn ListFolders>> + Send + Sync + 'static,
    ) -> Self {
        self.set_list_folders_builder(f);
        self
    }

    pub async fn build(self, account_config: AccountConfig) -> Result<BackendV2<C>> {
        #[cfg(feature = "folder-list")]
        let list_folders = self
            .context_builder
            .list_folders_builder()
            .or(self.list_folders_builder);

        let context = self.context_builder.build(&account_config).await?;
        let mut backend = BackendV2::new(account_config, context);

        #[cfg(feature = "folder-list")]
        if let Some(f) = list_folders {
            backend.set_list_folders(f(&backend.context));
        }

        Ok(backend)
    }
}

pub struct BackendV2<C: Send> {
    pub account_config: AccountConfig,
    pub context: C,

    #[cfg(feature = "folder-list")]
    pub list_folders: Option<Box<dyn ListFolders>>,
}

impl<C: Send> BackendV2<C> {
    pub fn new(account_config: AccountConfig, context: C) -> Self {
        Self {
            account_config,
            context,
            list_folders: None,
        }
    }

    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(&mut self, f: Option<Box<dyn ListFolders>>) {
        self.list_folders = f;
    }

    #[cfg(feature = "folder-list")]
    pub async fn list_folders(&self) -> Result<Folders> {
        self.list_folders
            .as_ref()
            .ok_or(Error::ListFoldersNotAvailableError)?
            .list_folders()
            .await
    }
}
