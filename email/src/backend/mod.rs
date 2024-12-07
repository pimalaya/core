//! # Backend
//!
//! A backend is a set of features like adding folder, listing
//! envelopes or sending message. This module exposes everything you
//! need to create your own backend.
//!
//! ## Dynamic backend
//!
//! A dynamic backend is composed of features defined at
//! runtime. Calling an undefined feature leads to a runtime
//! error. Such backend is useful when you do not know in advance
//! which feature is enabled or disabled (for example, from a user
//! configuration file).
//!
//! The simplest way to build a dynamic backend is to use the
//! [`BackendBuilder`]. It allows you to dynamically enable or disable
//! features using the builder pattern. The `build` method consumes
//! the builder to build the final backend. This module comes with two
//! backend implementations:
//!
//! - [`Backend`], a basic backend instance exposing features directly
//!
//! - [`BackendPool`], a backend where multiple contexts are
//! built and put in a pool, which allow you to execute features in
//! parallel
//!
//! You can create your own instance by implementing the
//! [`AsyncTryIntoBackendFeatures`] trait.
//!
//! See a full example at `../../tests/dynamic_backend.rs`.
//!
//! ## Static backend
//!
//! A static backend is composed of features defined at compilation
//! time. Such backend is useful when you know in advance which
//! feature should be enabled or disabled. It mostly relies on
//! traits. You will have to create your own backend instance as well
//! as manually implement backend features.
//!
//! See a full example at `../../tests/static_backend.rs`.

pub mod context;
mod error;
pub mod feature;
pub mod mapper;
pub mod macros {
    pub use email_macros::BackendContext;
}

#[cfg(feature = "sync")]
use std::hash::DefaultHasher;
use std::sync::Arc;

use async_trait::async_trait;
use paste::paste;
#[cfg(feature = "watch")]
use tokio::sync::oneshot::{Receiver, Sender};

#[doc(inline)]
pub use self::error::{Error, Result};
use self::{
    context::{BackendContext, BackendContextBuilder},
    feature::{BackendFeature, BackendFeatureSource, CheckUp},
};
#[cfg(feature = "watch")]
use crate::envelope::watch::WatchEnvelopes;
#[cfg(feature = "thread")]
use crate::envelope::{thread::ThreadEnvelopes, ThreadedEnvelopes};
#[cfg(feature = "sync")]
use crate::sync::hash::SyncHash;
use crate::{
    account::config::{AccountConfig, HasAccountConfig},
    envelope::{
        get::GetEnvelope,
        list::{ListEnvelopes, ListEnvelopesOptions},
        Envelope, Envelopes, Id, SingleId,
    },
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags, Flags},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder, Folders,
    },
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        peek::PeekMessages, r#move::MoveMessages, remove::RemoveMessages, send::SendMessage,
        Messages,
    },
    AnyResult,
};

/// The basic backend implementation.
///
/// This is the most primitive backend implementation: it owns its
/// context, and backend features are directly called from it.
///
/// This implementation is useful when you need to call features in
/// serie. If you need to call features in batch (parallel), see the
/// [`pool::BackendPool`] implementation instead.
pub struct Backend<C>
where
    C: BackendContext,
{
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,
    /// The backend context.
    pub context: Arc<C>,

    /// The add folder backend feature.
    pub add_folder: Option<BackendFeature<C, dyn AddFolder>>,
    /// The list folders backend feature.
    pub list_folders: Option<BackendFeature<C, dyn ListFolders>>,
    /// The expunge folder backend feature.
    pub expunge_folder: Option<BackendFeature<C, dyn ExpungeFolder>>,
    /// The purge folder backend feature.
    pub purge_folder: Option<BackendFeature<C, dyn PurgeFolder>>,
    /// The delete folder backend feature.
    pub delete_folder: Option<BackendFeature<C, dyn DeleteFolder>>,

    /// The get envelope backend feature.
    pub get_envelope: Option<BackendFeature<C, dyn GetEnvelope>>,
    /// The list envelopes backend feature.
    pub list_envelopes: Option<BackendFeature<C, dyn ListEnvelopes>>,
    /// The thread envelopes backend feature.
    #[cfg(feature = "thread")]
    pub thread_envelopes: Option<BackendFeature<C, dyn ThreadEnvelopes>>,
    /// The watch envelopes backend feature.
    #[cfg(feature = "watch")]
    pub watch_envelopes: Option<BackendFeature<C, dyn WatchEnvelopes>>,

    /// The add flags backend feature.
    pub add_flags: Option<BackendFeature<C, dyn AddFlags>>,
    /// The set flags backend feature.
    pub set_flags: Option<BackendFeature<C, dyn SetFlags>>,
    /// The remove flags backend feature.
    pub remove_flags: Option<BackendFeature<C, dyn RemoveFlags>>,

    /// The add message backend feature.
    pub add_message: Option<BackendFeature<C, dyn AddMessage>>,
    /// The send message backend feature.
    pub send_message: Option<BackendFeature<C, dyn SendMessage>>,
    /// The peek messages backend feature.
    pub peek_messages: Option<BackendFeature<C, dyn PeekMessages>>,
    /// The get messages backend feature.
    pub get_messages: Option<BackendFeature<C, dyn GetMessages>>,
    /// The copy messages backend feature.
    pub copy_messages: Option<BackendFeature<C, dyn CopyMessages>>,
    /// The move messages backend feature.
    pub move_messages: Option<BackendFeature<C, dyn MoveMessages>>,
    /// The delete messages backend feature.
    pub delete_messages: Option<BackendFeature<C, dyn DeleteMessages>>,
    /// The delete messages backend feature.
    pub remove_messages: Option<BackendFeature<C, dyn RemoveMessages>>,
}

impl<C: BackendContext> HasAccountConfig for Backend<C> {
    fn account_config(&self) -> &AccountConfig {
        &self.account_config
    }
}

#[async_trait]
impl<C: BackendContext> AddFolder for Backend<C> {
    async fn add_folder(&self, folder: &str) -> AnyResult<()> {
        self.add_folder
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::AddFolderNotAvailableError)?
            .add_folder(folder)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> ListFolders for Backend<C> {
    async fn list_folders(&self) -> AnyResult<Folders> {
        self.list_folders
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::ListFoldersNotAvailableError)?
            .list_folders()
            .await
    }
}

#[async_trait]
impl<C: BackendContext> ExpungeFolder for Backend<C> {
    async fn expunge_folder(&self, folder: &str) -> AnyResult<()> {
        self.expunge_folder
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::ExpungeFolderNotAvailableError)?
            .expunge_folder(folder)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> PurgeFolder for Backend<C> {
    async fn purge_folder(&self, folder: &str) -> AnyResult<()> {
        self.purge_folder
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::PurgeFolderNotAvailableError)?
            .purge_folder(folder)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> DeleteFolder for Backend<C> {
    async fn delete_folder(&self, folder: &str) -> AnyResult<()> {
        self.delete_folder
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::DeleteFolderNotAvailableError)?
            .delete_folder(folder)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> GetEnvelope for Backend<C> {
    async fn get_envelope(&self, folder: &str, id: &SingleId) -> AnyResult<Envelope> {
        self.get_envelope
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::GetEnvelopeNotAvailableError)?
            .get_envelope(folder, id)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> ListEnvelopes for Backend<C> {
    async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<Envelopes> {
        self.list_envelopes
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::ListEnvelopesNotAvailableError)?
            .list_envelopes(folder, opts)
            .await
    }
}

#[cfg(feature = "thread")]
#[async_trait]
impl<C: BackendContext> ThreadEnvelopes for Backend<C> {
    async fn thread_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        self.thread_envelopes
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::ThreadEnvelopesNotAvailableError)?
            .thread_envelopes(folder, opts)
            .await
    }

    async fn thread_envelope(
        &self,
        folder: &str,
        id: SingleId,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<ThreadedEnvelopes> {
        self.thread_envelopes
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::ThreadEnvelopesNotAvailableError)?
            .thread_envelope(folder, id, opts)
            .await
    }
}

#[cfg(feature = "watch")]
#[async_trait]
impl<C: BackendContext> WatchEnvelopes for Backend<C> {
    async fn watch_envelopes(
        &self,
        folder: &str,
        wait_for_shutdown_request: Receiver<()>,
        shutdown: Sender<()>,
    ) -> AnyResult<()> {
        self.watch_envelopes
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::WatchEnvelopesNotAvailableError)?
            .watch_envelopes(folder, wait_for_shutdown_request, shutdown)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> AddFlags for Backend<C> {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.add_flags
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::AddFlagsNotAvailableError)?
            .add_flags(folder, id, flags)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> SetFlags for Backend<C> {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.set_flags
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::SetFlagsNotAvailableError)?
            .set_flags(folder, id, flags)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> RemoveFlags for Backend<C> {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        self.remove_flags
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::RemoveFlagsNotAvailableError)?
            .remove_flags(folder, id, flags)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> AddMessage for Backend<C> {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        msg: &[u8],
        flags: &Flags,
    ) -> AnyResult<SingleId> {
        self.add_message
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::AddMessageNotAvailableError)?
            .add_message_with_flags(folder, msg, flags)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> SendMessage for Backend<C> {
    async fn send_message(&self, msg: &[u8]) -> AnyResult<()> {
        self.send_message
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::SendMessageNotAvailableError)?
            .send_message(msg)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> PeekMessages for Backend<C> {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        self.peek_messages
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::PeekMessagesNotAvailableError)?
            .peek_messages(folder, id)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> GetMessages for Backend<C> {
    async fn get_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        self.get_messages
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::GetMessagesNotAvailableError)?
            .get_messages(folder, id)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> CopyMessages for Backend<C> {
    async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        self.copy_messages
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::CopyMessagesNotAvailableError)?
            .copy_messages(from_folder, to_folder, id)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> MoveMessages for Backend<C> {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        self.move_messages
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::MoveMessagesNotAvailableError)?
            .move_messages(from_folder, to_folder, id)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> DeleteMessages for Backend<C> {
    async fn delete_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        self.delete_messages
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::DeleteMessagesNotAvailableError)?
            .delete_messages(folder, id)
            .await
    }
}

#[async_trait]
impl<C: BackendContext> RemoveMessages for Backend<C> {
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        self.remove_messages
            .as_ref()
            .and_then(|feature| feature(&self.context))
            .ok_or(Error::RemoveMessagesNotAvailableError)?
            .remove_messages(folder, id)
            .await
    }
}

/// Macro for defining [`BackendBuilder`] feature getter and setters.
macro_rules! feature_accessors {
    ($feat:ty) => {
        paste! {
            pub fn [<get_ $feat:snake>](
                &self
            ) -> Option<BackendFeature<CB::Context, dyn $feat>> {
                match &self.[<$feat:snake>] {
                    BackendFeatureSource::None => None,
                    BackendFeatureSource::Context => self.ctx_builder.[<$feat:snake>]().clone(),
                    BackendFeatureSource::Backend(f) => Some(f.clone()),
                }
            }

            /// Set the given backend feature.
            pub fn [<set_ $feat:snake>](
                &mut self,
                f: impl Into<BackendFeatureSource<CB::Context, dyn $feat>>,
            ) {
                self.[<$feat:snake>] = f.into();
            }

            /// Set the given backend feature, using the builder
            /// pattern.
            pub fn [<with_ $feat:snake>](
                mut self,
                f: impl Into<BackendFeatureSource<CB::Context, dyn $feat>>,
            ) -> Self {
                self.[<set_ $feat:snake>](f);
                self
            }

            /// Disable the given backend feature, using the builder
            /// pattern.
            pub fn [<without_ $feat:snake>](mut self) -> Self {
                self.[<set_ $feat:snake>](BackendFeatureSource::None);
                self
            }

            /// Use the given backend feature from the context
            /// builder, using the builder pattern.
            pub fn [<with_context_ $feat:snake>](mut self) -> Self {
                self.[<set_ $feat:snake>](BackendFeatureSource::Context);
                self
            }
        }
    };
}

/// The runtime backend builder.
///
/// The determination of backend's features occurs dynamically at
/// runtime, making the utilization of traits and generics potentially
/// less advantageous in this context. This consideration is
/// particularly relevant if the client interface is an interactive
/// shell (To Be Announced).
///
/// Furthermore, this design empowers the programmatic management of
/// features during runtime.
///
/// Alternatively, users have the option to define their custom
/// structs and implement the same traits as those implemented by
/// `BackendBuilder`. This approach allows for the creation of bespoke
/// functionality tailored to specific requirements.
pub struct BackendBuilder<CB>
where
    CB: BackendContextBuilder,
{
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,
    /// The backend context builder.
    pub ctx_builder: CB,

    /// The noop backend builder feature.
    pub check_up: BackendFeatureSource<CB::Context, dyn CheckUp>,

    /// The add folder backend builder feature.
    pub add_folder: BackendFeatureSource<CB::Context, dyn AddFolder>,
    /// The list folders backend builder feature.
    pub list_folders: BackendFeatureSource<CB::Context, dyn ListFolders>,
    /// The expunge folder backend builder feature.
    pub expunge_folder: BackendFeatureSource<CB::Context, dyn ExpungeFolder>,
    /// The purge folder backend builder feature.
    pub purge_folder: BackendFeatureSource<CB::Context, dyn PurgeFolder>,
    /// The delete folder backend builder feature.
    pub delete_folder: BackendFeatureSource<CB::Context, dyn DeleteFolder>,

    /// The get envelope backend builder feature.
    pub get_envelope: BackendFeatureSource<CB::Context, dyn GetEnvelope>,
    /// The list envelopes backend builder feature.
    pub list_envelopes: BackendFeatureSource<CB::Context, dyn ListEnvelopes>,
    /// The thread envelopes backend builder feature.
    #[cfg(feature = "thread")]
    pub thread_envelopes: BackendFeatureSource<CB::Context, dyn ThreadEnvelopes>,
    /// The watch envelopes backend builder feature.
    #[cfg(feature = "watch")]
    pub watch_envelopes: BackendFeatureSource<CB::Context, dyn WatchEnvelopes>,

    /// The add flags backend builder feature.
    pub add_flags: BackendFeatureSource<CB::Context, dyn AddFlags>,
    /// The set flags backend builder feature.
    pub set_flags: BackendFeatureSource<CB::Context, dyn SetFlags>,
    /// The remove flags backend builder feature.
    pub remove_flags: BackendFeatureSource<CB::Context, dyn RemoveFlags>,

    /// The add message backend builder feature.
    pub add_message: BackendFeatureSource<CB::Context, dyn AddMessage>,
    /// The send message backend builder feature.
    pub send_message: BackendFeatureSource<CB::Context, dyn SendMessage>,
    /// The peek messages backend builder feature.
    pub peek_messages: BackendFeatureSource<CB::Context, dyn PeekMessages>,
    /// The get messages backend builder feature.
    pub get_messages: BackendFeatureSource<CB::Context, dyn GetMessages>,
    /// The copy messages backend builder feature.
    pub copy_messages: BackendFeatureSource<CB::Context, dyn CopyMessages>,
    /// The move messages backend builder feature.
    pub move_messages: BackendFeatureSource<CB::Context, dyn MoveMessages>,
    /// The delete messages backend builder feature.
    pub delete_messages: BackendFeatureSource<CB::Context, dyn DeleteMessages>,
    /// The remove messages backend builder feature.
    pub remove_messages: BackendFeatureSource<CB::Context, dyn RemoveMessages>,
}

impl<CB> BackendBuilder<CB>
where
    CB: BackendContextBuilder,
{
    feature_accessors!(CheckUp);
    feature_accessors!(AddFolder);
    feature_accessors!(ListFolders);
    feature_accessors!(ExpungeFolder);
    feature_accessors!(PurgeFolder);
    feature_accessors!(DeleteFolder);
    feature_accessors!(GetEnvelope);
    feature_accessors!(ListEnvelopes);
    #[cfg(feature = "thread")]
    feature_accessors!(ThreadEnvelopes);
    #[cfg(feature = "watch")]
    feature_accessors!(WatchEnvelopes);
    feature_accessors!(AddFlags);
    feature_accessors!(SetFlags);
    feature_accessors!(RemoveFlags);
    feature_accessors!(AddMessage);
    feature_accessors!(SendMessage);
    feature_accessors!(PeekMessages);
    feature_accessors!(GetMessages);
    feature_accessors!(CopyMessages);
    feature_accessors!(MoveMessages);
    feature_accessors!(DeleteMessages);
    feature_accessors!(RemoveMessages);

    /// Create a new backend builder using the given backend context
    /// builder.
    ///
    /// All features are taken from the context by default.
    pub fn new(account_config: Arc<AccountConfig>, ctx_builder: CB) -> Self {
        Self {
            account_config,
            ctx_builder,

            check_up: BackendFeatureSource::Context,

            add_folder: BackendFeatureSource::Context,
            list_folders: BackendFeatureSource::Context,
            expunge_folder: BackendFeatureSource::Context,
            purge_folder: BackendFeatureSource::Context,
            delete_folder: BackendFeatureSource::Context,

            get_envelope: BackendFeatureSource::Context,
            list_envelopes: BackendFeatureSource::Context,
            #[cfg(feature = "thread")]
            thread_envelopes: BackendFeatureSource::Context,
            #[cfg(feature = "watch")]
            watch_envelopes: BackendFeatureSource::Context,

            add_flags: BackendFeatureSource::Context,
            set_flags: BackendFeatureSource::Context,
            remove_flags: BackendFeatureSource::Context,

            add_message: BackendFeatureSource::Context,
            send_message: BackendFeatureSource::Context,
            peek_messages: BackendFeatureSource::Context,
            get_messages: BackendFeatureSource::Context,
            copy_messages: BackendFeatureSource::Context,
            move_messages: BackendFeatureSource::Context,
            delete_messages: BackendFeatureSource::Context,
            remove_messages: BackendFeatureSource::Context,
        }
    }

    /// Disable all features for this backend builder.
    pub fn without_features(mut self) -> Self {
        self.set_list_folders(BackendFeatureSource::None);
        self
    }

    pub async fn check_up(self) -> AnyResult<()> {
        let ctx = self.ctx_builder.clone().build().await?;
        match self.get_check_up().and_then(move |f| f(&ctx)) {
            Some(f) => f.check_up().await,
            None => Ok(()),
        }
    }

    pub async fn build(self) -> AnyResult<Backend<CB::Context>> {
        let add_folder = self.get_add_folder();
        let list_folders = self.get_list_folders();
        let expunge_folder = self.get_expunge_folder();
        let purge_folder = self.get_purge_folder();
        let delete_folder = self.get_delete_folder();

        let get_envelope = self.get_get_envelope();
        let list_envelopes = self.get_list_envelopes();
        #[cfg(feature = "thread")]
        let thread_envelopes = self.get_thread_envelopes();
        #[cfg(feature = "watch")]
        let watch_envelopes = self.get_watch_envelopes();

        let add_flags = self.get_add_flags();
        let set_flags = self.get_set_flags();
        let remove_flags = self.get_remove_flags();

        let add_message = self.get_add_message();
        let send_message = self.get_send_message();
        let peek_messages = self.get_peek_messages();
        let get_messages = self.get_get_messages();
        let copy_messages = self.get_copy_messages();
        let move_messages = self.get_move_messages();
        let delete_messages = self.get_delete_messages();
        let remove_messages = self.get_remove_messages();

        Ok(Backend {
            account_config: self.account_config,
            context: Arc::new(self.ctx_builder.build().await?),

            add_folder,
            list_folders,
            expunge_folder,
            purge_folder,
            delete_folder,

            get_envelope,
            list_envelopes,
            #[cfg(feature = "thread")]
            thread_envelopes,
            #[cfg(feature = "watch")]
            watch_envelopes,

            add_flags,
            set_flags,
            remove_flags,

            add_message,
            send_message,
            peek_messages,
            get_messages,
            copy_messages,
            move_messages,
            delete_messages,
            remove_messages,
        })
    }
}

#[async_trait]
impl<CB> Clone for BackendBuilder<CB>
where
    CB: BackendContextBuilder,
{
    fn clone(&self) -> Self {
        Self {
            account_config: self.account_config.clone(),
            ctx_builder: self.ctx_builder.clone(),

            check_up: self.check_up.clone(),

            add_folder: self.add_folder.clone(),
            list_folders: self.list_folders.clone(),
            expunge_folder: self.expunge_folder.clone(),
            purge_folder: self.purge_folder.clone(),
            delete_folder: self.delete_folder.clone(),

            get_envelope: self.get_envelope.clone(),
            list_envelopes: self.list_envelopes.clone(),
            #[cfg(feature = "thread")]
            thread_envelopes: self.thread_envelopes.clone(),
            #[cfg(feature = "watch")]
            watch_envelopes: self.watch_envelopes.clone(),

            add_flags: self.add_flags.clone(),
            set_flags: self.set_flags.clone(),
            remove_flags: self.remove_flags.clone(),

            add_message: self.add_message.clone(),
            send_message: self.send_message.clone(),
            peek_messages: self.peek_messages.clone(),
            get_messages: self.get_messages.clone(),
            copy_messages: self.copy_messages.clone(),
            move_messages: self.move_messages.clone(),
            delete_messages: self.delete_messages.clone(),
            remove_messages: self.remove_messages.clone(),
        }
    }
}

#[cfg(feature = "sync")]
impl<CB> SyncHash for BackendBuilder<CB>
where
    CB: BackendContextBuilder + SyncHash,
{
    fn sync_hash(&self, state: &mut DefaultHasher) {
        self.ctx_builder.sync_hash(state)
    }
}
