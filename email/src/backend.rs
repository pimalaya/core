//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the
//! [AccountConfiguration](crate::account::config::AccountConfig).

pub mod macros {
    pub use email_macros::BackendContext;
}

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

// BACKEND V2 STARTS HERE

/// Optional dynamic boxed backend feature.
pub type SomeBackendFeature<F> = Option<Box<F>>;

/// Thread-safe backend feature builder.
///
/// The backend feature builder is a function that takes a reference
/// to a context in parameter and return an optional dynamic boxed
/// backend feature.
pub type BackendFeatureBuilder<C, F> = dyn Fn(&C) -> SomeBackendFeature<F> + Send + Sync;

/// The backend context trait.
///
/// This is just a marker for other traits. Every backend context
/// needs to implement this trait manually or to derive
/// [`BackendContext`].
pub trait BackendContext: Send {}

/// Get a context in a context.
///
/// A good use case is when you have a custom backend context composed
/// of multiple subcontexts:
///
/// ```rust
/// struct MyContext {
///     imap: email::imap::ImapContextSync,
///     smtp: email::smtp::SmtpContextSync,
/// }
/// ```
///
/// If your context is composed of optional subcontexts, use
/// [`FindBackendSubcontext`] instead.
pub trait GetBackendSubcontext<C: BackendContext> {
    fn get_subcontext(&self) -> &C;
}

/// Generic implementation for contexts that match themselves as
/// subcontext.
impl<C: BackendContext> GetBackendSubcontext<C> for C {
    fn get_subcontext(&self) -> &C {
        self
    }
}

/// Find a context in a context.
///
/// A good use case is when you have a custom backend context composed
/// of multiple optional subcontexts:
///
/// ```rust
/// struct MyContext {
///     imap: Option<email::imap::ImapContextSync>,
///     smtp: Option<email::smtp::SmtpContextSync>,
/// }
/// ```
///
/// If your context is composed of existing subcontexts, use
/// [`GetBackendSubcontext`] instead.
pub trait FindBackendSubcontext<C: BackendContext> {
    fn find_subcontext(&self) -> Option<&C>;
}

/// Generic implementation for contexts that match themselves as
/// subcontext.
///
/// If a context can get a subcontext, then it can also find a
/// subcontext.
impl<C: BackendContext, T: GetBackendSubcontext<C>> FindBackendSubcontext<C> for T {
    fn find_subcontext(&self) -> Option<&C> {
        Some(self.get_subcontext())
    }
}

/// Map a feature from a subcontext to a context.
///
/// A good use case is when you have a custom backend context composed
/// of multiple subcontexts. When implementing the
/// [`BackendContextBuilder`] trait for your custom backend context,
/// you will have to forward backend features using the right
/// subcontext.
///
/// ```rust
/// use async_trait::async_trait;
///
/// use email::imap::ImapContextSync;
/// use email::smtp::SmtpContextSync;
/// use email::backend::BackendContextBuilder;
///
/// struct MyContext {
///     imap: Option<ImapContextSync>,
///     smtp: Option<SmtpContextSync>,
/// }
///
/// impl FindBackendSubcontext<ImapContextSync> for MyContext {
///     fn find_subcontext(&self) -> Option<&ImapContextSync> {
///         self.imap.as_ref()
///     }
/// }
///
/// impl FindBackendSubcontext<SmtpContextSync> for MyContext {
///     fn find_subcontext(&self) -> Option<&SmtpContextSync> {
///         self.smtp.as_ref()
///     }
/// }
///
/// #[derive(Clone)]
/// struct MyContextBuilder {
///     imap: Option<ImapContextBuilder>,
///     smtp: Option<SmtpContextBuilder>,
/// }
///
/// #[async_trait]
/// impl BackendContextBuilder for MyContextBuilder {
///     type Context = MyContext;
///
///     fn list_folders(&self) -> SomeBackendFeatureBuilder<Self::Context, dyn ListFolders> {
///         // This is how you can map a
///         // `SomeBackendFeatureBuilder<ImapContextSync, dyn ListFolders>` to a
///         // `SomeBackendFeatureBuilder<Self::Context, dyn ListFolders>`:
///         self.list_folders_from(self.imap.as_ref())
///     }
///
///     async fn build(self, account_config: &AccountConfig) -> Result<Self::Context> {
///         let imap = match self.imap {
///             Some(imap) => Some(BackendContextBuilder::build(imap, account_config).await?),
///             None => None,
///         };
///
///         let smtp = match self.smtp {
///             Some(smtp) => Some(BackendContextBuilder::build(smtp, account_config).await?),
///             None => None,
///         };
///
///         Ok(MyContext { imap, smtp })
///     }
/// }
/// ```
///
pub trait MapBackendFeature<B>
where
    Self: BackendContextBuilderV2,
    Self::Context: FindBackendSubcontext<B::Context> + 'static,
    B: BackendContextBuilderV2,
    B::Context: BackendContext + 'static,
{
    fn map_feature<T: ?Sized + 'static>(
        &self,
        f: Option<Arc<BackendFeatureBuilder<B::Context, T>>>,
    ) -> Option<Arc<BackendFeatureBuilder<Self::Context, T>>> {
        let f = f?;
        Some(Arc::new(move |ctx| f(ctx.find_subcontext()?)))
    }

    #[cfg(feature = "folder-list")]
    fn list_folders_from(
        &self,
        cb: Option<&B>,
    ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn ListFolders>>> {
        self.map_feature(cb.and_then(|cb| cb.list_folders()))
    }
}

/// Generic implementation for the backend context builder with a
/// context implementing [`FindBackendSubcontext`].
impl<T, B> MapBackendFeature<B> for T
where
    T: BackendContextBuilderV2,
    T::Context: FindBackendSubcontext<B::Context> + 'static,
    B: BackendContextBuilderV2,
    B::Context: BackendContext + 'static,
{
}

/// The backend context builder trait.
///
/// This trait defines how a context should be built. It also defines
/// default backend features implemented by the context.
#[async_trait]
pub trait BackendContextBuilderV2: Clone + Send + Sync {
    /// The type of the context being built by the builder.
    ///
    /// The context needs to implement [`Send`], as it is sent accross
    /// asynchronous tasks. Wrapping your context in a
    /// [`std::sync::Arc`] should be enough. If your context needs to
    /// be mutated, you can also wrap it in a
    /// [`tokio::sync::Mutex`]. See existing implementations of
    /// `email::imap::ImapContextSync` or
    /// `email::smtp::SmtpContextSync`.
    type Context: BackendContext;

    /// Define the add folder backend feature builder.
    #[cfg(feature = "folder-add")]
    fn add_folder(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn AddFolder>>> {
        None
    }

    /// Define the list folders backend feature builder.
    #[cfg(feature = "folder-list")]
    fn list_folders(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn ListFolders>>> {
        None
    }

    /// Define the expunge folder backend feature builder.
    #[cfg(feature = "folder-expunge")]
    fn expunge_folder(
        &self,
    ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn ExpungeFolder>>> {
        None
    }

    /// Define the purge folder backend feature builder.
    #[cfg(feature = "folder-purge")]
    fn purge_folder(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn PurgeFolder>>> {
        None
    }

    /// Define the delete folder backend feature builder.
    #[cfg(feature = "folder-delete")]
    fn delete_folder(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn DeleteFolder>>> {
        None
    }

    /// Define the list envelopes backend feature builder.
    #[cfg(feature = "envelope-list")]
    fn list_envelopes(
        &self,
    ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn ListEnvelopes>>> {
        None
    }

    /// Define the watch envelopes backend feature builder.
    #[cfg(feature = "envelope-watch")]
    fn watch_envelopes(
        &self,
    ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn WatchEnvelopes>>> {
        None
    }

    /// Define the get envelope backend feature builder.
    #[cfg(feature = "envelope-get")]
    fn get_envelope(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn GetEnvelope>>> {
        None
    }

    /// Define the add flags backend feature builder.
    #[cfg(feature = "flag-add")]
    fn add_flags(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn AddFlags>>> {
        None
    }

    /// Define the set flags backend feature builder.
    #[cfg(feature = "flag-set")]
    fn set_flags(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn SetFlags>>> {
        None
    }

    /// Define the remove flags backend feature builder.
    #[cfg(feature = "flag-remove")]
    fn remove_flags(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn RemoveFlags>>> {
        None
    }

    /// Define the add message backend feature builder.
    #[cfg(feature = "message-add")]
    fn add_message(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn AddMessage>>> {
        None
    }

    /// Define the send message backend feature builder.
    #[cfg(feature = "message-send")]
    fn send_message(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn SendMessage>>> {
        None
    }

    /// Define the peek messages backend feature builder.
    #[cfg(feature = "message-peek")]
    fn peek_messages(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn PeekMessages>>> {
        None
    }

    /// Define the get messages backend feature builder.
    #[cfg(feature = "message-get")]
    fn get_messages(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn GetMessages>>> {
        None
    }

    /// Define the copy messages backend feature builder.
    #[cfg(feature = "message-copy")]
    fn copy_messages(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn CopyMessages>>> {
        None
    }

    /// Define the move messages backend feature builder.
    #[cfg(feature = "message-move")]
    fn move_messages(&self) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn MoveMessages>>> {
        None
    }

    /// Define the delete messages backend feature builder.
    #[cfg(feature = "message-delete")]
    fn delete_messages(
        &self,
    ) -> Option<Arc<BackendFeatureBuilder<Self::Context, dyn DeleteMessages>>> {
        None
    }

    /// Build the final context.
    async fn build(self, account_config: &AccountConfig) -> Result<Self::Context>;
}

/// The runtime backend builder.
///
/// This backend helps you to build a backend with features set up at
/// runtime rather than at compile time.
pub struct BackendBuilderV2<B: BackendContextBuilderV2> {
    /// The backend context builder.
    ctx_builder: B,

    /// The add folder backend feature builder.
    #[cfg(feature = "folder-add")]
    add_folder: Option<Arc<BackendFeatureBuilder<B::Context, dyn AddFolder>>>,

    /// The list folders backend feature builder.
    #[cfg(feature = "folder-list")]
    list_folders: Option<Arc<BackendFeatureBuilder<B::Context, dyn ListFolders>>>,

    /// The expunge folder backend feature builder.
    #[cfg(feature = "folder-expunge")]
    expunge_folder: Option<Arc<BackendFeatureBuilder<B::Context, dyn ExpungeFolder>>>,

    /// The purge folder backend feature builder.
    #[cfg(feature = "folder-purge")]
    purge_folder: Option<Arc<BackendFeatureBuilder<B::Context, dyn PurgeFolder>>>,

    /// The delete folder backend feature builder.
    #[cfg(feature = "folder-delete")]
    delete_folder: Option<Arc<BackendFeatureBuilder<B::Context, dyn DeleteFolder>>>,

    /// The list envelopes backend feature builder.
    #[cfg(feature = "envelope-list")]
    list_envelopes: Option<Arc<BackendFeatureBuilder<B::Context, dyn ListEnvelopes>>>,

    /// The watch envelopes backend feature builder.
    #[cfg(feature = "envelope-watch")]
    watch_envelopes: Option<Arc<BackendFeatureBuilder<B::Context, dyn WatchEnvelopes>>>,

    /// The get envelope backend feature builder.
    #[cfg(feature = "envelope-get")]
    get_envelope: Option<Arc<BackendFeatureBuilder<B::Context, dyn GetEnvelope>>>,

    /// The add flags backend feature builder.
    #[cfg(feature = "flag-add")]
    add_flags: Option<Arc<BackendFeatureBuilder<B::Context, dyn AddFlags>>>,

    /// The set flags backend feature builder.
    #[cfg(feature = "flag-set")]
    set_flags: Option<Arc<BackendFeatureBuilder<B::Context, dyn SetFlags>>>,

    /// The remove flags backend feature builder.
    #[cfg(feature = "flag-remove")]
    remove_flags: Option<Arc<BackendFeatureBuilder<B::Context, dyn RemoveFlags>>>,

    /// The add message backend feature builder.
    #[cfg(feature = "message-add")]
    add_message: Option<Arc<BackendFeatureBuilder<B::Context, dyn AddMessage>>>,

    /// The send message backend feature builder.
    #[cfg(feature = "message-send")]
    send_message: Option<Arc<BackendFeatureBuilder<B::Context, dyn SendMessage>>>,

    /// The peek messages backend feature builder.
    #[cfg(feature = "message-peek")]
    peek_messages: Option<Arc<BackendFeatureBuilder<B::Context, dyn PeekMessages>>>,

    /// The get messages backend feature builder.
    #[cfg(feature = "message-get")]
    get_messages: Option<Arc<BackendFeatureBuilder<B::Context, dyn GetMessages>>>,

    /// The copy messages backend feature builder.
    #[cfg(feature = "message-copy")]
    copy_messages: Option<Arc<BackendFeatureBuilder<B::Context, dyn CopyMessages>>>,

    /// The move messages backend feature builder.
    #[cfg(feature = "message-move")]
    move_messages: Option<Arc<BackendFeatureBuilder<B::Context, dyn MoveMessages>>>,

    /// The delete messages backend feature builder.
    #[cfg(feature = "message-delete")]
    delete_messages: Option<Arc<BackendFeatureBuilder<B::Context, dyn DeleteMessages>>>,
}

impl<B: BackendContextBuilderV2> BackendBuilderV2<B> {
    /// Build a new backend builder using the given backend context
    /// builder.
    ///
    /// All features are disabled by default.
    pub fn new(ctx_builder: B) -> Self {
        Self {
            ctx_builder,

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

            #[cfg(feature = "message-send")]
            send_message: None,

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
        }
    }

    /// Set the add folder backend feature builder.
    #[cfg(feature = "folder-add")]
    pub fn set_add_folder(&mut self, f: &'static BackendFeatureBuilder<B::Context, dyn AddFolder>) {
        self.add_folder = Some(Arc::new(f));
    }

    /// Set the add folder backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "folder-add")]
    pub fn with_add_folder(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn AddFolder>,
    ) -> Self {
        self.set_add_folder(f);
        self
    }

    /// Set the list folders backend feature builder.
    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn ListFolders>,
    ) {
        self.list_folders = Some(Arc::new(f));
    }

    /// Set the list folders backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "folder-list")]
    pub fn with_list_folders(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn ListFolders>,
    ) -> Self {
        self.set_list_folders(f);
        self
    }

    /// Set the expunge folder backend feature builder.
    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn ExpungeFolder>,
    ) {
        self.expunge_folder = Some(Arc::new(f));
    }

    /// Set the expunge folder backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "folder-expunge")]
    pub fn with_expunge_folder(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn ExpungeFolder>,
    ) -> Self {
        self.set_expunge_folder(f);
        self
    }

    /// Set the purge folder backend feature builder.
    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn PurgeFolder>,
    ) {
        self.purge_folder = Some(Arc::new(f));
    }

    /// Set the purge folder backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "folder-purge")]
    pub fn with_purge_folder(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn PurgeFolder>,
    ) -> Self {
        self.set_purge_folder(f);
        self
    }

    /// Set the delete folder backend feature builder.
    #[cfg(feature = "folder-delete")]
    pub fn set_delete_folder(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn DeleteFolder>,
    ) {
        self.delete_folder = Some(Arc::new(f));
    }

    /// Set the delete folder backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "folder-delete")]
    pub fn with_delete_folder(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn DeleteFolder>,
    ) -> Self {
        self.set_delete_folder(f);
        self
    }

    /// Set the list envelopes backend feature builder.
    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn ListEnvelopes>,
    ) {
        self.list_envelopes = Some(Arc::new(f));
    }

    /// Set the list envelopes backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "envelope-list")]
    pub fn with_list_envelopes(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn ListEnvelopes>,
    ) -> Self {
        self.set_list_envelopes(f);
        self
    }

    /// Set the watch envelopes backend feature builder.
    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn WatchEnvelopes>,
    ) {
        self.watch_envelopes = Some(Arc::new(f));
    }

    /// Set the watch envelopes backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "envelope-watch")]
    pub fn with_watch_envelopes(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn WatchEnvelopes>,
    ) -> Self {
        self.set_watch_envelopes(f);
        self
    }

    /// Set the get envelope backend feature builder.
    #[cfg(feature = "envelope-get")]
    pub fn set_get_envelope(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn GetEnvelope>,
    ) {
        self.get_envelope = Some(Arc::new(f));
    }

    /// Set the get envelope backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "envelope-get")]
    pub fn with_get_envelope(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn GetEnvelope>,
    ) -> Self {
        self.set_get_envelope(f);
        self
    }

    /// Set the add flags backend feature builder.
    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(&mut self, f: &'static BackendFeatureBuilder<B::Context, dyn AddFlags>) {
        self.add_flags = Some(Arc::new(f));
    }

    /// Set the add flags backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "flag-add")]
    pub fn with_add_flags(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn AddFlags>,
    ) -> Self {
        self.set_add_flags(f);
        self
    }

    /// Set the set flags backend feature builder.
    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(&mut self, f: &'static BackendFeatureBuilder<B::Context, dyn SetFlags>) {
        self.set_flags = Some(Arc::new(f));
    }

    /// Set the set flags backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "flag-set")]
    pub fn with_set_flags(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn SetFlags>,
    ) -> Self {
        self.set_set_flags(f);
        self
    }

    /// Set the remove flags backend feature builder.
    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn RemoveFlags>,
    ) {
        self.remove_flags = Some(Arc::new(f));
    }

    /// Set the remove flags backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "flag-remove")]
    pub fn with_remove_flags(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn RemoveFlags>,
    ) -> Self {
        self.set_remove_flags(f);
        self
    }

    /// Set the add message backend feature builder.
    #[cfg(feature = "message-add")]
    pub fn set_add_message(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn AddMessage>,
    ) {
        self.add_message = Some(Arc::new(f));
    }

    /// Set the add message backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "message-add")]
    pub fn with_add_message(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn AddMessage>,
    ) -> Self {
        self.set_add_message(f);
        self
    }

    /// Set the send message backend feature builder.
    #[cfg(feature = "message-send")]
    pub fn set_send_message(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn SendMessage>,
    ) {
        self.send_message = Some(Arc::new(f));
    }

    /// Set the send message backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "message-send")]
    pub fn with_send_message(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn SendMessage>,
    ) -> Self {
        self.set_send_message(f);
        self
    }

    /// Set the peek messages backend feature builder.
    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn PeekMessages>,
    ) {
        self.peek_messages = Some(Arc::new(f));
    }

    /// Set the peek messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-peek")]
    pub fn with_peek_messages(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn PeekMessages>,
    ) -> Self {
        self.set_peek_messages(f);
        self
    }

    /// Set the get messages backend feature builder.
    #[cfg(feature = "message-get")]
    pub fn set_get_messages(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn GetMessages>,
    ) {
        self.get_messages = Some(Arc::new(f));
    }

    /// Set the get messages backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "message-get")]
    pub fn with_get_messages(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn GetMessages>,
    ) -> Self {
        self.set_get_messages(f);
        self
    }

    /// Set the copy messages backend feature builder.
    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn CopyMessages>,
    ) {
        self.copy_messages = Some(Arc::new(f));
    }

    /// Set the copy messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-copy")]
    pub fn with_copy_messages(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn CopyMessages>,
    ) -> Self {
        self.set_copy_messages(f);
        self
    }

    /// Set the move messages backend feature builder.
    #[cfg(feature = "message-move")]
    pub fn set_move_messages(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn MoveMessages>,
    ) {
        self.move_messages = Some(Arc::new(f));
    }

    /// Set the move messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-move")]
    pub fn with_move_messages(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn MoveMessages>,
    ) -> Self {
        self.set_move_messages(f);
        self
    }

    /// Set the delete messages backend feature builder.
    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(
        &mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn DeleteMessages>,
    ) {
        self.delete_messages = Some(Arc::new(f));
    }

    /// Set the delete messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-delete")]
    pub fn with_delete_messages(
        mut self,
        f: &'static BackendFeatureBuilder<B::Context, dyn DeleteMessages>,
    ) -> Self {
        self.set_delete_messages(f);
        self
    }

    /// Build the final backend.
    pub async fn build(self, account_config: AccountConfig) -> Result<BackendV2<B::Context>> {
        #[cfg(feature = "folder-add")]
        let add_folder = self.ctx_builder.add_folder().or(self.add_folder);

        #[cfg(feature = "folder-list")]
        let list_folders = self.ctx_builder.list_folders().or(self.list_folders);

        #[cfg(feature = "folder-expunge")]
        let expunge_folder = self.ctx_builder.expunge_folder().or(self.expunge_folder);

        #[cfg(feature = "folder-purge")]
        let purge_folder = self.ctx_builder.purge_folder().or(self.purge_folder);

        #[cfg(feature = "folder-delete")]
        let delete_folder = self.ctx_builder.delete_folder().or(self.delete_folder);

        #[cfg(feature = "envelope-list")]
        let list_envelopes = self.ctx_builder.list_envelopes().or(self.list_envelopes);

        #[cfg(feature = "envelope-watch")]
        let watch_envelopes = self.ctx_builder.watch_envelopes().or(self.watch_envelopes);

        #[cfg(feature = "envelope-get")]
        let get_envelope = self.ctx_builder.get_envelope().or(self.get_envelope);

        #[cfg(feature = "flag-add")]
        let add_flags = self.ctx_builder.add_flags().or(self.add_flags);

        #[cfg(feature = "flag-set")]
        let set_flags = self.ctx_builder.set_flags().or(self.set_flags);

        #[cfg(feature = "flag-remove")]
        let remove_flags = self.ctx_builder.remove_flags().or(self.remove_flags);

        #[cfg(feature = "message-add")]
        let add_message = self.ctx_builder.add_message().or(self.add_message);

        #[cfg(feature = "message-send")]
        let send_message = self.ctx_builder.send_message().or(self.send_message);

        #[cfg(feature = "message-peek")]
        let peek_messages = self.ctx_builder.peek_messages().or(self.peek_messages);

        #[cfg(feature = "message-get")]
        let get_messages = self.ctx_builder.get_messages().or(self.get_messages);

        #[cfg(feature = "message-copy")]
        let copy_messages = self.ctx_builder.copy_messages().or(self.copy_messages);

        #[cfg(feature = "message-move")]
        let move_messages = self.ctx_builder.move_messages().or(self.move_messages);

        #[cfg(feature = "message-delete")]
        let delete_messages = self.ctx_builder.delete_messages().or(self.delete_messages);

        let context = self.ctx_builder.build(&account_config).await?;
        let mut backend = BackendV2::new(account_config, context);

        #[cfg(feature = "folder-add")]
        backend.set_add_folder(add_folder.and_then(|f| f(&backend.context)));

        #[cfg(feature = "folder-list")]
        backend.set_list_folders(list_folders.and_then(|f| f(&backend.context)));

        #[cfg(feature = "folder-expunge")]
        backend.set_expunge_folder(expunge_folder.and_then(|f| f(&backend.context)));

        #[cfg(feature = "folder-purge")]
        backend.set_purge_folder(purge_folder.and_then(|f| f(&backend.context)));

        #[cfg(feature = "folder-delete")]
        backend.set_delete_folder(delete_folder.and_then(|f| f(&backend.context)));

        #[cfg(feature = "envelope-list")]
        backend.set_list_envelopes(list_envelopes.and_then(|f| f(&backend.context)));

        #[cfg(feature = "envelope-watch")]
        backend.set_watch_envelopes(watch_envelopes.and_then(|f| f(&backend.context)));

        #[cfg(feature = "envelope-get")]
        backend.set_get_envelope(get_envelope.and_then(|f| f(&backend.context)));

        #[cfg(feature = "flag-add")]
        backend.set_add_flags(add_flags.and_then(|f| f(&backend.context)));

        #[cfg(feature = "flag-set")]
        backend.set_set_flags(set_flags.and_then(|f| f(&backend.context)));

        #[cfg(feature = "flag-remove")]
        backend.set_remove_flags(remove_flags.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-add")]
        backend.set_add_message(add_message.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-send")]
        backend.set_send_message(send_message.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-peek")]
        backend.set_peek_messages(peek_messages.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-get")]
        backend.set_get_messages(get_messages.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-copy")]
        backend.set_copy_messages(copy_messages.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-move")]
        backend.set_move_messages(move_messages.and_then(|f| f(&backend.context)));

        #[cfg(feature = "message-delete")]
        backend.set_delete_messages(delete_messages.and_then(|f| f(&backend.context)));

        Ok(backend)
    }
}

/// The email backend.
///
/// The backend owns a context, as well as multiple optional backend
/// features.
pub struct BackendV2<C: BackendContext> {
    /// The account configuration.
    pub account_config: AccountConfig,

    /// The backend context.
    pub context: C,

    /// The optional add folder feature.
    #[cfg(feature = "folder-add")]
    pub add_folder: SomeBackendFeature<dyn AddFolder>,

    /// The optional list folders feature.
    #[cfg(feature = "folder-list")]
    pub list_folders: SomeBackendFeature<dyn ListFolders>,

    /// The optional expunge folder feature.
    #[cfg(feature = "folder-expunge")]
    pub expunge_folder: SomeBackendFeature<dyn ExpungeFolder>,

    /// The optional purge folder feature.
    #[cfg(feature = "folder-purge")]
    pub purge_folder: SomeBackendFeature<dyn PurgeFolder>,

    /// The optional delete folder feature.
    #[cfg(feature = "folder-delete")]
    pub delete_folder: SomeBackendFeature<dyn DeleteFolder>,

    /// The optional list envelopes feature.
    #[cfg(feature = "envelope-list")]
    pub list_envelopes: SomeBackendFeature<dyn ListEnvelopes>,

    /// The optional watch envelopes feature.
    #[cfg(feature = "envelope-watch")]
    pub watch_envelopes: SomeBackendFeature<dyn WatchEnvelopes>,

    /// The optional get envelope feature.
    #[cfg(feature = "envelope-get")]
    pub get_envelope: SomeBackendFeature<dyn GetEnvelope>,

    /// The optional add flags feature.
    #[cfg(feature = "flag-add")]
    pub add_flags: SomeBackendFeature<dyn AddFlags>,

    /// The optional set flags feature.
    #[cfg(feature = "flag-set")]
    pub set_flags: SomeBackendFeature<dyn SetFlags>,

    /// The optional remove flags feature.
    #[cfg(feature = "flag-remove")]
    pub remove_flags: SomeBackendFeature<dyn RemoveFlags>,

    /// The optional add message feature.
    #[cfg(feature = "message-add")]
    pub add_message: SomeBackendFeature<dyn AddMessage>,

    /// The optional send message feature.
    #[cfg(feature = "message-send")]
    pub send_message: SomeBackendFeature<dyn SendMessage>,

    /// The optional peek messages feature.
    #[cfg(feature = "message-peek")]
    pub peek_messages: SomeBackendFeature<dyn PeekMessages>,

    /// The optional get messages feature.
    #[cfg(feature = "message-get")]
    pub get_messages: SomeBackendFeature<dyn GetMessages>,

    /// The optional copy messages feature.
    #[cfg(feature = "message-copy")]
    pub copy_messages: SomeBackendFeature<dyn CopyMessages>,

    /// The optional move messages feature.
    #[cfg(feature = "message-move")]
    pub move_messages: SomeBackendFeature<dyn MoveMessages>,

    /// The optional delete messages feature.
    #[cfg(feature = "message-delete")]
    pub delete_messages: SomeBackendFeature<dyn DeleteMessages>,
}

impl<C: BackendContext> BackendV2<C> {
    /// Build a new backend from an account configuration and a
    /// context.
    pub fn new(account_config: AccountConfig, context: C) -> Self {
        Self {
            account_config,

            context,

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

            #[cfg(feature = "message-send")]
            send_message: None,

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
        }
    }

    /// Set the add folder backend feature.
    #[cfg(feature = "folder-add")]
    pub fn set_add_folder(&mut self, f: SomeBackendFeature<dyn AddFolder>) {
        self.add_folder = f;
    }

    /// Set the list folders backend feature.
    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(&mut self, f: SomeBackendFeature<dyn ListFolders>) {
        self.list_folders = f;
    }
    /// Set the expunge folder backend feature.
    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(&mut self, f: SomeBackendFeature<dyn ExpungeFolder>) {
        self.expunge_folder = f;
    }

    /// Set the purge folder backend feature.
    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(&mut self, f: SomeBackendFeature<dyn PurgeFolder>) {
        self.purge_folder = f;
    }

    /// Set the delete folder backend feature.
    #[cfg(feature = "folder-delete")]
    pub fn set_delete_folder(&mut self, f: SomeBackendFeature<dyn DeleteFolder>) {
        self.delete_folder = f;
    }

    /// Set the list envelopes backend feature.
    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(&mut self, f: SomeBackendFeature<dyn ListEnvelopes>) {
        self.list_envelopes = f;
    }

    /// Set the watch envelopes backend feature.
    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(&mut self, f: SomeBackendFeature<dyn WatchEnvelopes>) {
        self.watch_envelopes = f;
    }

    /// Set the get envelope backend feature.
    #[cfg(feature = "envelope-get")]
    pub fn set_get_envelope(&mut self, f: SomeBackendFeature<dyn GetEnvelope>) {
        self.get_envelope = f;
    }

    /// Set the add flags backend feature.
    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(&mut self, f: SomeBackendFeature<dyn AddFlags>) {
        self.add_flags = f;
    }

    /// Set the set flags backend feature.
    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(&mut self, f: SomeBackendFeature<dyn SetFlags>) {
        self.set_flags = f;
    }

    /// Set the remove flags backend feature.
    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(&mut self, f: SomeBackendFeature<dyn RemoveFlags>) {
        self.remove_flags = f;
    }

    /// Set the add message backend feature.
    #[cfg(feature = "message-add")]
    pub fn set_add_message(&mut self, f: SomeBackendFeature<dyn AddMessage>) {
        self.add_message = f;
    }

    /// Set the send message backend feature.
    #[cfg(feature = "message-send")]
    pub fn set_send_message(&mut self, f: SomeBackendFeature<dyn SendMessage>) {
        self.send_message = f;
    }

    /// Set the peek messages backend feature.
    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(&mut self, f: SomeBackendFeature<dyn PeekMessages>) {
        self.peek_messages = f;
    }

    /// Set the get messages backend feature.
    #[cfg(feature = "message-get")]
    pub fn set_get_messages(&mut self, f: SomeBackendFeature<dyn GetMessages>) {
        self.get_messages = f;
    }

    /// Set the copy messages backend feature.
    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(&mut self, f: SomeBackendFeature<dyn CopyMessages>) {
        self.copy_messages = f;
    }

    /// Set the move messages backend feature.
    #[cfg(feature = "message-move")]
    pub fn set_move_messages(&mut self, f: SomeBackendFeature<dyn MoveMessages>) {
        self.move_messages = f;
    }

    /// Set the delete messages backend feature.
    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(&mut self, f: SomeBackendFeature<dyn DeleteMessages>) {
        self.delete_messages = f;
    }

    /// Call the add folder feature, returning an error if the feature
    /// is not defined.
    #[cfg(feature = "folder-add")]
    pub async fn add_folder(&self, folder: &str) -> Result<()> {
        self.add_folder
            .as_ref()
            .ok_or(Error::AddFolderNotAvailableError)?
            .add_folder(folder)
            .await
    }

    /// Call the list folders feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "folder-list")]
    pub async fn list_folders(&self) -> Result<Folders> {
        self.list_folders
            .as_ref()
            .ok_or(Error::ListFoldersNotAvailableError)?
            .list_folders()
            .await
    }

    /// Call the expunge folder feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "folder-expunge")]
    pub async fn expunge_folder(&self, folder: &str) -> Result<()> {
        self.expunge_folder
            .as_ref()
            .ok_or(Error::ExpungeFolderNotAvailableError)?
            .expunge_folder(folder)
            .await
    }

    /// Call the purge folder feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "folder-purge")]
    pub async fn purge_folder(&self, folder: &str) -> Result<()> {
        self.purge_folder
            .as_ref()
            .ok_or(Error::PurgeFolderNotAvailableError)?
            .purge_folder(folder)
            .await
    }

    /// Call the delete folder feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "folder-delete")]
    pub async fn delete_folder(&self, folder: &str) -> Result<()> {
        self.delete_folder
            .as_ref()
            .ok_or(Error::DeleteFolderNotAvailableError)?
            .delete_folder(folder)
            .await
    }

    /// Call the list envelopes feature, returning an error if the
    /// feature is not defined.
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

    /// Call the watch envelopes feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "envelope-watch")]
    pub async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        self.watch_envelopes
            .as_ref()
            .ok_or(Error::WatchEnvelopesNotAvailableError)?
            .watch_envelopes(folder)
            .await
    }

    /// Call the get envelope feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "envelope-get")]
    pub async fn get_envelope(&self, folder: &str, id: &Id) -> Result<Envelope> {
        self.get_envelope
            .as_ref()
            .ok_or(Error::GetEnvelopeNotAvailableError)?
            .get_envelope(folder, id)
            .await
    }

    /// Call the add flags feature, returning an error if the feature
    /// is not defined.
    #[cfg(feature = "flag-add")]
    pub async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.add_flags
            .as_ref()
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flags(folder, id, flags)
            .await
    }

    /// Call the add flag feature, returning an error if the feature
    /// is not defined.
    #[cfg(feature = "flag-add")]
    pub async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.add_flags
            .as_ref()
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flag(folder, id, flag)
            .await
    }

    /// Call the set flags feature, returning an error if the feature
    /// is not defined.
    #[cfg(feature = "flag-set")]
    pub async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.set_flags
            .as_ref()
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flags(folder, id, flags)
            .await
    }

    /// Call the set flag feature, returning an error if the feature
    /// is not defined.
    #[cfg(feature = "flag-set")]
    pub async fn set_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.set_flags
            .as_ref()
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flag(folder, id, flag)
            .await
    }

    /// Call the remove flags feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "flag-remove")]
    pub async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flags(folder, id, flags)
            .await
    }

    /// Call the remove flag feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "flag-remove")]
    pub async fn remove_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.remove_flags
            .as_ref()
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flag(folder, id, flag)
            .await
    }

    /// Call the add message with flags feature, returning an error if
    /// the feature is not defined.
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

    /// Call the add message with flag feature, returning an error if
    /// the feature is not defined.
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

    /// Call the add message feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "message-add")]
    pub async fn add_message(&self, folder: &str, raw_msg: &[u8]) -> Result<SingleId> {
        self.add_message
            .as_ref()
            .ok_or(Error::AddMessageWithFlagsNotAvailableError)?
            .add_message(folder, raw_msg)
            .await
    }

    /// Call the send message feature, returning an error if the
    /// feature is not defined.
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

    /// Call the peek messages feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "message-peek")]
    pub async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        self.peek_messages
            .as_ref()
            .ok_or(Error::PeekMessagesNotAvailableError)?
            .peek_messages(folder, id)
            .await
    }

    /// Call the get messages feature, returning an error if the
    /// feature is not defined.
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

    /// Call the copy messages feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "message-copy")]
    pub async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.copy_messages
            .as_ref()
            .ok_or(Error::CopyMessagesNotAvailableError)?
            .copy_messages(from_folder, to_folder, id)
            .await
    }

    /// Call the move messages feature, returning an error if the
    /// feature is not defined.
    #[cfg(feature = "message-move")]
    pub async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.move_messages
            .as_ref()
            .ok_or(Error::MoveMessagesNotAvailableError)?
            .move_messages(from_folder, to_folder, id)
            .await
    }

    /// Call the delete messages feature, returning an error if the
    /// feature is not defined.
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
}
