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
#[cfg(feature = "message-delete")]
use crate::message::delete::DeleteMessages;
#[cfg(feature = "message-get")]
use crate::message::get::GetMessages;
#[cfg(feature = "message-peek")]
use crate::message::peek::PeekMessages;
#[cfg(feature = "message-move")]
use crate::message::r#move::MoveMessages;
#[cfg(feature = "message-send")]
use crate::message::send::SendMessage;
#[cfg(feature = "sync")]
use crate::thread_pool::{ThreadPoolContext, ThreadPoolContextBuilder};
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
    #[cfg(feature = "message-send")]
    #[error("cannot send message: feature not available")]
    SendMessageNotAvailableError,
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
}

/// Optional dynamic boxed backend feature.
pub type BackendFeature<F> = Option<Box<F>>;

/// Thread-safe backend feature builder.
///
/// The backend feature builder is a function that takes a reference
/// to a context in parameter and return an optional dynamic boxed
/// backend feature.
pub type BackendFeatureBuilder<C, F> = Option<Arc<dyn Fn(&C) -> BackendFeature<F> + Send + Sync>>;

/// The backend context trait.
///
/// This is just a marker for other traits. Every backend context
/// needs to implement this trait manually or to derive
/// [`BackendContext`].
pub trait BackendContext: Send + Sync {
    //
}

#[cfg(feature = "sync")]
impl<C: BackendContext> ThreadPoolContext for Backend<C> {
    //
}

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
/// use std::sync::Arc;
/// use async_trait::async_trait;
///
/// use email::imap::{ImapContextSync, ImapContextBuilder};
/// use email::smtp::{SmtpContextSync, SmtpContextBuilder};
/// use email::backend::{BackendContextBuilder, FindBackendSubcontext, BackendFeatureBuilder, MapBackendFeature, macros::BackendContext};
/// use email::account::config::AccountConfig;
/// use email::folder::list::ListFolders;
/// use email::Result;
///
/// #[derive(BackendContext)]
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
///     fn list_folders(&self) -> BackendFeatureBuilder<Self::Context, dyn ListFolders> {
///         // This is how you can map a
///         // `BackendFeatureBuilder<ImapContextSync, dyn ListFolders>` to a
///         // `BackendFeatureBuilder<Self::Context, dyn ListFolders>`:
///         self.list_folders_from(self.imap.as_ref())
///     }
///
///     async fn build(self) -> Result<Self::Context> {
///         let imap = match self.imap {
///             Some(imap) => Some(imap.build().await?),
///             None => None,
///         };
///
///         let smtp = match self.smtp {
///             Some(smtp) => Some(smtp.build().await?),
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
    Self: BackendContextBuilder,
    Self::Context: FindBackendSubcontext<B::Context> + 'static,
    B: BackendContextBuilder,
    B::Context: BackendContext + 'static,
{
    fn map_feature<T: ?Sized + 'static>(
        &self,
        f: BackendFeatureBuilder<B::Context, T>,
    ) -> BackendFeatureBuilder<Self::Context, T> {
        let f = f?;
        Some(Arc::new(move |ctx| f(ctx.find_subcontext()?)))
    }

    #[cfg(feature = "folder-add")]
    fn add_folder_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn AddFolder> {
        self.map_feature(cb.and_then(|cb| cb.add_folder()))
    }

    #[cfg(feature = "folder-list")]
    fn list_folders_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn ListFolders> {
        self.map_feature(cb.and_then(|cb| cb.list_folders()))
    }

    #[cfg(feature = "folder-expunge")]
    fn expunge_folder_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn ExpungeFolder> {
        self.map_feature(cb.and_then(|cb| cb.expunge_folder()))
    }

    #[cfg(feature = "folder-purge")]
    fn purge_folder_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn PurgeFolder> {
        self.map_feature(cb.and_then(|cb| cb.purge_folder()))
    }

    #[cfg(feature = "folder-delete")]
    fn delete_folder_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn DeleteFolder> {
        self.map_feature(cb.and_then(|cb| cb.delete_folder()))
    }

    #[cfg(feature = "envelope-get")]
    fn get_envelope_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn GetEnvelope> {
        self.map_feature(cb.and_then(|cb| cb.get_envelope()))
    }

    #[cfg(feature = "envelope-list")]
    fn list_envelopes_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn ListEnvelopes> {
        self.map_feature(cb.and_then(|cb| cb.list_envelopes()))
    }

    #[cfg(feature = "envelope-watch")]
    fn watch_envelopes_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn WatchEnvelopes> {
        self.map_feature(cb.and_then(|cb| cb.watch_envelopes()))
    }

    #[cfg(feature = "flag-add")]
    fn add_flags_from(&self, cb: Option<&B>) -> BackendFeatureBuilder<Self::Context, dyn AddFlags> {
        self.map_feature(cb.and_then(|cb| cb.add_flags()))
    }

    #[cfg(feature = "flag-set")]
    fn set_flags_from(&self, cb: Option<&B>) -> BackendFeatureBuilder<Self::Context, dyn SetFlags> {
        self.map_feature(cb.and_then(|cb| cb.set_flags()))
    }

    #[cfg(feature = "flag-remove")]
    fn remove_flags_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn RemoveFlags> {
        self.map_feature(cb.and_then(|cb| cb.remove_flags()))
    }

    #[cfg(feature = "message-add")]
    fn add_message_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn AddMessage> {
        self.map_feature(cb.and_then(|cb| cb.add_message()))
    }

    #[cfg(feature = "message-send")]
    fn send_message_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn SendMessage> {
        self.map_feature(cb.and_then(|cb| cb.send_message()))
    }

    #[cfg(feature = "message-get")]
    fn get_messages_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn GetMessages> {
        self.map_feature(cb.and_then(|cb| cb.get_messages()))
    }

    #[cfg(feature = "message-peek")]
    fn peek_messages_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn PeekMessages> {
        self.map_feature(cb.and_then(|cb| cb.peek_messages()))
    }

    #[cfg(feature = "message-copy")]
    fn copy_messages_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn CopyMessages> {
        self.map_feature(cb.and_then(|cb| cb.copy_messages()))
    }

    #[cfg(feature = "message-move")]
    fn move_messages_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn MoveMessages> {
        self.map_feature(cb.and_then(|cb| cb.move_messages()))
    }

    #[cfg(feature = "message-delete")]
    fn delete_messages_from(
        &self,
        cb: Option<&B>,
    ) -> BackendFeatureBuilder<Self::Context, dyn DeleteMessages> {
        self.map_feature(cb.and_then(|cb| cb.delete_messages()))
    }
}

/// Generic implementation for the backend context builder with a
/// context implementing [`FindBackendSubcontext`].
impl<T, B> MapBackendFeature<B> for T
where
    T: BackendContextBuilder,
    T::Context: FindBackendSubcontext<B::Context> + 'static,
    B: BackendContextBuilder,
    B::Context: BackendContext + 'static,
{
}

/// The backend context builder trait.
///
/// This trait defines how a context should be built. It also defines
/// default backend features implemented by the context.
#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
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
    fn add_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFolder> {
        None
    }

    /// Define the list folders backend feature builder.
    #[cfg(feature = "folder-list")]
    fn list_folders(&self) -> BackendFeatureBuilder<Self::Context, dyn ListFolders> {
        None
    }

    /// Define the expunge folder backend feature builder.
    #[cfg(feature = "folder-expunge")]
    fn expunge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn ExpungeFolder> {
        None
    }

    /// Define the purge folder backend feature builder.
    #[cfg(feature = "folder-purge")]
    fn purge_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn PurgeFolder> {
        None
    }

    /// Define the delete folder backend feature builder.
    #[cfg(feature = "folder-delete")]
    fn delete_folder(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteFolder> {
        None
    }

    /// Define the list envelopes backend feature builder.
    #[cfg(feature = "envelope-list")]
    fn list_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn ListEnvelopes> {
        None
    }

    /// Define the watch envelopes backend feature builder.
    #[cfg(feature = "envelope-watch")]
    fn watch_envelopes(&self) -> BackendFeatureBuilder<Self::Context, dyn WatchEnvelopes> {
        None
    }

    /// Define the get envelope backend feature builder.
    #[cfg(feature = "envelope-get")]
    fn get_envelope(&self) -> BackendFeatureBuilder<Self::Context, dyn GetEnvelope> {
        None
    }

    /// Define the add flags backend feature builder.
    #[cfg(feature = "flag-add")]
    fn add_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn AddFlags> {
        None
    }

    /// Define the set flags backend feature builder.
    #[cfg(feature = "flag-set")]
    fn set_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn SetFlags> {
        None
    }

    /// Define the remove flags backend feature builder.
    #[cfg(feature = "flag-remove")]
    fn remove_flags(&self) -> BackendFeatureBuilder<Self::Context, dyn RemoveFlags> {
        None
    }

    /// Define the add message backend feature builder.
    #[cfg(feature = "message-add")]
    fn add_message(&self) -> BackendFeatureBuilder<Self::Context, dyn AddMessage> {
        None
    }

    /// Define the send message backend feature builder.
    #[cfg(feature = "message-send")]
    fn send_message(&self) -> BackendFeatureBuilder<Self::Context, dyn SendMessage> {
        None
    }

    /// Define the peek messages backend feature builder.
    #[cfg(feature = "message-peek")]
    fn peek_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn PeekMessages> {
        None
    }

    /// Define the get messages backend feature builder.
    #[cfg(feature = "message-get")]
    fn get_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn GetMessages> {
        None
    }

    /// Define the copy messages backend feature builder.
    #[cfg(feature = "message-copy")]
    fn copy_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn CopyMessages> {
        None
    }

    /// Define the move messages backend feature builder.
    #[cfg(feature = "message-move")]
    fn move_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn MoveMessages> {
        None
    }

    /// Define the delete messages backend feature builder.
    #[cfg(feature = "message-delete")]
    fn delete_messages(&self) -> BackendFeatureBuilder<Self::Context, dyn DeleteMessages> {
        None
    }

    /// Build the final context.
    async fn build(self) -> Result<Self::Context>;
}

#[cfg(feature = "sync")]
#[async_trait]
impl<B: BackendContextBuilder> ThreadPoolContextBuilder for BackendBuilder<B> {
    type Context = Backend<B::Context>;

    async fn build(self) -> Result<Self::Context> {
        BackendBuilder::build(self).await
    }
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
pub struct BackendBuilder<B: BackendContextBuilder> {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The backend context builder.
    ctx_builder: B,

    /// The add folder backend feature builder.
    #[cfg(feature = "folder-add")]
    add_folder: Option<BackendFeatureBuilder<B::Context, dyn AddFolder>>,

    /// The list folders backend feature builder.
    #[cfg(feature = "folder-list")]
    list_folders: Option<BackendFeatureBuilder<B::Context, dyn ListFolders>>,

    /// The expunge folder backend feature builder.
    #[cfg(feature = "folder-expunge")]
    expunge_folder: Option<BackendFeatureBuilder<B::Context, dyn ExpungeFolder>>,

    /// The purge folder backend feature builder.
    #[cfg(feature = "folder-purge")]
    purge_folder: Option<BackendFeatureBuilder<B::Context, dyn PurgeFolder>>,

    /// The delete folder backend feature builder.
    #[cfg(feature = "folder-delete")]
    delete_folder: Option<BackendFeatureBuilder<B::Context, dyn DeleteFolder>>,

    /// The list envelopes backend feature builder.
    #[cfg(feature = "envelope-list")]
    list_envelopes: Option<BackendFeatureBuilder<B::Context, dyn ListEnvelopes>>,

    /// The watch envelopes backend feature builder.
    #[cfg(feature = "envelope-watch")]
    watch_envelopes: Option<BackendFeatureBuilder<B::Context, dyn WatchEnvelopes>>,

    /// The get envelope backend feature builder.
    #[cfg(feature = "envelope-get")]
    get_envelope: Option<BackendFeatureBuilder<B::Context, dyn GetEnvelope>>,

    /// The add flags backend feature builder.
    #[cfg(feature = "flag-add")]
    add_flags: Option<BackendFeatureBuilder<B::Context, dyn AddFlags>>,

    /// The set flags backend feature builder.
    #[cfg(feature = "flag-set")]
    set_flags: Option<BackendFeatureBuilder<B::Context, dyn SetFlags>>,

    /// The remove flags backend feature builder.
    #[cfg(feature = "flag-remove")]
    remove_flags: Option<BackendFeatureBuilder<B::Context, dyn RemoveFlags>>,

    /// The add message backend feature builder.
    #[cfg(feature = "message-add")]
    add_message: Option<BackendFeatureBuilder<B::Context, dyn AddMessage>>,

    /// The send message backend feature builder.
    #[cfg(feature = "message-send")]
    send_message: Option<BackendFeatureBuilder<B::Context, dyn SendMessage>>,

    /// The peek messages backend feature builder.
    #[cfg(feature = "message-peek")]
    peek_messages: Option<BackendFeatureBuilder<B::Context, dyn PeekMessages>>,

    /// The get messages backend feature builder.
    #[cfg(feature = "message-get")]
    get_messages: Option<BackendFeatureBuilder<B::Context, dyn GetMessages>>,

    /// The copy messages backend feature builder.
    #[cfg(feature = "message-copy")]
    copy_messages: Option<BackendFeatureBuilder<B::Context, dyn CopyMessages>>,

    /// The move messages backend feature builder.
    #[cfg(feature = "message-move")]
    move_messages: Option<BackendFeatureBuilder<B::Context, dyn MoveMessages>>,

    /// The delete messages backend feature builder.
    #[cfg(feature = "message-delete")]
    delete_messages: Option<BackendFeatureBuilder<B::Context, dyn DeleteMessages>>,
}

impl<B: BackendContextBuilder> BackendBuilder<B> {
    /// Build a new backend builder using the given backend context
    /// builder.
    ///
    /// All features are disabled by default.
    pub fn new(account_config: Arc<AccountConfig>, ctx_builder: B) -> Self {
        Self {
            account_config,
            ctx_builder,

            #[cfg(feature = "folder-add")]
            add_folder: Some(None),

            #[cfg(feature = "folder-list")]
            list_folders: Some(None),

            #[cfg(feature = "folder-expunge")]
            expunge_folder: Some(None),

            #[cfg(feature = "folder-purge")]
            purge_folder: Some(None),

            #[cfg(feature = "folder-delete")]
            delete_folder: Some(None),

            #[cfg(feature = "envelope-list")]
            list_envelopes: Some(None),

            #[cfg(feature = "envelope-watch")]
            watch_envelopes: Some(None),

            #[cfg(feature = "envelope-get")]
            get_envelope: Some(None),

            #[cfg(feature = "flag-add")]
            add_flags: Some(None),

            #[cfg(feature = "flag-set")]
            set_flags: Some(None),

            #[cfg(feature = "flag-remove")]
            remove_flags: Some(None),

            #[cfg(feature = "message-add")]
            add_message: Some(None),

            #[cfg(feature = "message-send")]
            send_message: Some(None),

            #[cfg(feature = "message-peek")]
            peek_messages: Some(None),

            #[cfg(feature = "message-get")]
            get_messages: Some(None),

            #[cfg(feature = "message-copy")]
            copy_messages: Some(None),

            #[cfg(feature = "message-move")]
            move_messages: Some(None),

            #[cfg(feature = "message-delete")]
            delete_messages: Some(None),
        }
    }

    pub fn with_default_features_disabled(mut self) -> Self {
        #[cfg(feature = "folder-add")]
        {
            self.add_folder = None;
        }

        #[cfg(feature = "folder-list")]
        {
            self.list_folders = None;
        }

        #[cfg(feature = "folder-expunge")]
        {
            self.expunge_folder = None;
        }

        #[cfg(feature = "folder-purge")]
        {
            self.purge_folder = None;
        }

        #[cfg(feature = "folder-delete")]
        {
            self.delete_folder = None;
        }

        #[cfg(feature = "envelope-list")]
        {
            self.list_envelopes = None;
        }

        #[cfg(feature = "envelope-watch")]
        {
            self.watch_envelopes = None;
        }

        #[cfg(feature = "envelope-get")]
        {
            self.get_envelope = None;
        }

        #[cfg(feature = "flag-add")]
        {
            self.add_flags = None;
        }

        #[cfg(feature = "flag-set")]
        {
            self.set_flags = None;
        }

        #[cfg(feature = "flag-remove")]
        {
            self.remove_flags = None;
        }

        #[cfg(feature = "message-add")]
        {
            self.add_message = None;
        }

        #[cfg(feature = "message-send")]
        {
            self.send_message = None;
        }

        #[cfg(feature = "message-peek")]
        {
            self.peek_messages = None;
        }

        #[cfg(feature = "message-get")]
        {
            self.get_messages = None;
        }

        #[cfg(feature = "message-copy")]
        {
            self.copy_messages = None;
        }

        #[cfg(feature = "message-move")]
        {
            self.move_messages = None;
        }

        #[cfg(feature = "message-delete")]
        {
            self.delete_messages = None;
        }

        self
    }

    /// Set the add folder backend feature builder.
    #[cfg(feature = "folder-add")]
    pub fn set_add_folder(&mut self, f: Option<BackendFeatureBuilder<B::Context, dyn AddFolder>>) {
        self.add_folder = f;
    }

    /// Set the add folder backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "folder-add")]
    pub fn with_add_folder(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn AddFolder>>,
    ) -> Self {
        self.set_add_folder(f);
        self
    }

    /// Set the list folders backend feature builder.
    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn ListFolders>>,
    ) {
        self.list_folders = f;
    }

    /// Set the list folders backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "folder-list")]
    pub fn with_list_folders(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn ListFolders>>,
    ) -> Self {
        self.set_list_folders(f);
        self
    }

    /// Set the expunge folder backend feature builder.
    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn ExpungeFolder>>,
    ) {
        self.expunge_folder = f;
    }

    /// Set the expunge folder backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "folder-expunge")]
    pub fn with_expunge_folder(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn ExpungeFolder>>,
    ) -> Self {
        self.set_expunge_folder(f);
        self
    }

    /// Set the purge folder backend feature builder.
    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn PurgeFolder>>,
    ) {
        self.purge_folder = f;
    }

    /// Set the purge folder backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "folder-purge")]
    pub fn with_purge_folder(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn PurgeFolder>>,
    ) -> Self {
        self.set_purge_folder(f);
        self
    }

    /// Set the delete folder backend feature builder.
    #[cfg(feature = "folder-delete")]
    pub fn set_delete_folder(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn DeleteFolder>>,
    ) {
        self.delete_folder = f;
    }

    /// Set the delete folder backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "folder-delete")]
    pub fn with_delete_folder(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn DeleteFolder>>,
    ) -> Self {
        self.set_delete_folder(f);
        self
    }

    /// Set the list envelopes backend feature builder.
    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn ListEnvelopes>>,
    ) {
        self.list_envelopes = f;
    }

    /// Set the list envelopes backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "envelope-list")]
    pub fn with_list_envelopes(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn ListEnvelopes>>,
    ) -> Self {
        self.set_list_envelopes(f);
        self
    }

    /// Set the watch envelopes backend feature builder.
    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn WatchEnvelopes>>,
    ) {
        self.watch_envelopes = f;
    }

    /// Set the watch envelopes backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "envelope-watch")]
    pub fn with_watch_envelopes(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn WatchEnvelopes>>,
    ) -> Self {
        self.set_watch_envelopes(f);
        self
    }

    /// Set the get envelope backend feature builder.
    #[cfg(feature = "envelope-get")]
    pub fn set_get_envelope(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn GetEnvelope>>,
    ) {
        self.get_envelope = f;
    }

    /// Set the get envelope backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "envelope-get")]
    pub fn with_get_envelope(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn GetEnvelope>>,
    ) -> Self {
        self.set_get_envelope(f);
        self
    }

    /// Set the add flags backend feature builder.
    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(&mut self, f: Option<BackendFeatureBuilder<B::Context, dyn AddFlags>>) {
        self.add_flags = f;
    }

    /// Set the add flags backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "flag-add")]
    pub fn with_add_flags(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn AddFlags>>,
    ) -> Self {
        self.set_add_flags(f);
        self
    }

    /// Set the set flags backend feature builder.
    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(&mut self, f: Option<BackendFeatureBuilder<B::Context, dyn SetFlags>>) {
        self.set_flags = f;
    }

    /// Set the set flags backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "flag-set")]
    pub fn with_set_flags(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn SetFlags>>,
    ) -> Self {
        self.set_set_flags(f);
        self
    }

    /// Set the remove flags backend feature builder.
    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn RemoveFlags>>,
    ) {
        self.remove_flags = f;
    }

    /// Set the remove flags backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "flag-remove")]
    pub fn with_remove_flags(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn RemoveFlags>>,
    ) -> Self {
        self.set_remove_flags(f);
        self
    }

    /// Set the add message backend feature builder.
    #[cfg(feature = "message-add")]
    pub fn set_add_message(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn AddMessage>>,
    ) {
        self.add_message = f;
    }

    /// Set the add message backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "message-add")]
    pub fn with_add_message(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn AddMessage>>,
    ) -> Self {
        self.set_add_message(f);
        self
    }

    /// Set the send message backend feature builder.
    #[cfg(feature = "message-send")]
    pub fn set_send_message(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn SendMessage>>,
    ) {
        self.send_message = f;
    }

    /// Set the send message backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "message-send")]
    pub fn with_send_message(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn SendMessage>>,
    ) -> Self {
        self.set_send_message(f);
        self
    }

    /// Set the peek messages backend feature builder.
    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn PeekMessages>>,
    ) {
        self.peek_messages = f;
    }

    /// Set the peek messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-peek")]
    pub fn with_peek_messages(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn PeekMessages>>,
    ) -> Self {
        self.set_peek_messages(f);
        self
    }

    /// Set the get messages backend feature builder.
    #[cfg(feature = "message-get")]
    pub fn set_get_messages(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn GetMessages>>,
    ) {
        self.get_messages = f;
    }

    /// Set the get messages backend feature builder using the builder
    /// pattern.
    #[cfg(feature = "message-get")]
    pub fn with_get_messages(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn GetMessages>>,
    ) -> Self {
        self.set_get_messages(f);
        self
    }

    /// Set the copy messages backend feature builder.
    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn CopyMessages>>,
    ) {
        self.copy_messages = f;
    }

    /// Set the copy messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-copy")]
    pub fn with_copy_messages(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn CopyMessages>>,
    ) -> Self {
        self.set_copy_messages(f);
        self
    }

    /// Set the move messages backend feature builder.
    #[cfg(feature = "message-move")]
    pub fn set_move_messages(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn MoveMessages>>,
    ) {
        self.move_messages = f;
    }

    /// Set the move messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-move")]
    pub fn with_move_messages(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn MoveMessages>>,
    ) -> Self {
        self.set_move_messages(f);
        self
    }

    /// Set the delete messages backend feature builder.
    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(
        &mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn DeleteMessages>>,
    ) {
        self.delete_messages = f;
    }

    /// Set the delete messages backend feature builder using the
    /// builder pattern.
    #[cfg(feature = "message-delete")]
    pub fn with_delete_messages(
        mut self,
        f: Option<BackendFeatureBuilder<B::Context, dyn DeleteMessages>>,
    ) -> Self {
        self.set_delete_messages(f);
        self
    }

    /// Build the final backend.
    pub async fn build(self) -> Result<Backend<B::Context>> {
        #[cfg(feature = "folder-add")]
        let add_folder = self
            .add_folder
            .and_then(|f| f.or(self.ctx_builder.add_folder()));

        #[cfg(feature = "folder-list")]
        let list_folders = self
            .list_folders
            .and_then(|f| f.or(self.ctx_builder.list_folders()));

        #[cfg(feature = "folder-expunge")]
        let expunge_folder = self
            .expunge_folder
            .and_then(|f| f.or(self.ctx_builder.expunge_folder()));

        #[cfg(feature = "folder-purge")]
        let purge_folder = self
            .purge_folder
            .and_then(|f| f.or(self.ctx_builder.purge_folder()));

        #[cfg(feature = "folder-delete")]
        let delete_folder = self
            .delete_folder
            .and_then(|f| f.or(self.ctx_builder.delete_folder()));

        #[cfg(feature = "envelope-list")]
        let list_envelopes = self
            .list_envelopes
            .and_then(|f| f.or(self.ctx_builder.list_envelopes()));

        #[cfg(feature = "envelope-watch")]
        let watch_envelopes = self
            .watch_envelopes
            .and_then(|f| f.or(self.ctx_builder.watch_envelopes()));

        #[cfg(feature = "envelope-get")]
        let get_envelope = self
            .get_envelope
            .and_then(|f| f.or(self.ctx_builder.get_envelope()));

        #[cfg(feature = "flag-add")]
        let add_flags = self
            .add_flags
            .and_then(|f| f.or(self.ctx_builder.add_flags()));

        #[cfg(feature = "flag-set")]
        let set_flags = self
            .set_flags
            .and_then(|f| f.or(self.ctx_builder.set_flags()));

        #[cfg(feature = "flag-remove")]
        let remove_flags = self
            .remove_flags
            .and_then(|f| f.or(self.ctx_builder.remove_flags()));

        #[cfg(feature = "message-add")]
        let add_message = self
            .add_message
            .and_then(|f| f.or(self.ctx_builder.add_message()));

        #[cfg(feature = "message-send")]
        let send_message = self
            .send_message
            .and_then(|f| f.or(self.ctx_builder.send_message()));

        #[cfg(feature = "message-peek")]
        let peek_messages = self
            .peek_messages
            .and_then(|f| f.or(self.ctx_builder.peek_messages()));

        #[cfg(feature = "message-get")]
        let get_messages = self
            .get_messages
            .and_then(|f| f.or(self.ctx_builder.get_messages()));

        #[cfg(feature = "message-copy")]
        let copy_messages = self
            .copy_messages
            .and_then(|f| f.or(self.ctx_builder.copy_messages()));

        #[cfg(feature = "message-move")]
        let move_messages = self
            .move_messages
            .and_then(|f| f.or(self.ctx_builder.move_messages()));

        #[cfg(feature = "message-delete")]
        let delete_messages = self
            .delete_messages
            .and_then(|f| f.or(self.ctx_builder.delete_messages()));

        let context = self.ctx_builder.build().await?;
        let mut backend = Backend::new(self.account_config, context);

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

impl<B: BackendContextBuilder + Clone> Clone for BackendBuilder<B> {
    fn clone(&self) -> Self {
        Self {
            account_config: self.account_config.clone(),
            ctx_builder: self.ctx_builder.clone(),

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
            #[cfg(feature = "message-send")]
            send_message: self.send_message.clone(),
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
        }
    }
}

/// The email backend.
///
/// The backend owns a context, as well as multiple optional backend
/// features.
pub struct Backend<C: BackendContext> {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The backend context.
    pub context: C,

    /// The optional add folder feature.
    #[cfg(feature = "folder-add")]
    pub add_folder: BackendFeature<dyn AddFolder>,

    /// The optional list folders feature.
    #[cfg(feature = "folder-list")]
    pub list_folders: BackendFeature<dyn ListFolders>,

    /// The optional expunge folder feature.
    #[cfg(feature = "folder-expunge")]
    pub expunge_folder: BackendFeature<dyn ExpungeFolder>,

    /// The optional purge folder feature.
    #[cfg(feature = "folder-purge")]
    pub purge_folder: BackendFeature<dyn PurgeFolder>,

    /// The optional delete folder feature.
    #[cfg(feature = "folder-delete")]
    pub delete_folder: BackendFeature<dyn DeleteFolder>,

    /// The optional list envelopes feature.
    #[cfg(feature = "envelope-list")]
    pub list_envelopes: BackendFeature<dyn ListEnvelopes>,

    /// The optional watch envelopes feature.
    #[cfg(feature = "envelope-watch")]
    pub watch_envelopes: BackendFeature<dyn WatchEnvelopes>,

    /// The optional get envelope feature.
    #[cfg(feature = "envelope-get")]
    pub get_envelope: BackendFeature<dyn GetEnvelope>,

    /// The optional add flags feature.
    #[cfg(feature = "flag-add")]
    pub add_flags: BackendFeature<dyn AddFlags>,

    /// The optional set flags feature.
    #[cfg(feature = "flag-set")]
    pub set_flags: BackendFeature<dyn SetFlags>,

    /// The optional remove flags feature.
    #[cfg(feature = "flag-remove")]
    pub remove_flags: BackendFeature<dyn RemoveFlags>,

    /// The optional add message feature.
    #[cfg(feature = "message-add")]
    pub add_message: BackendFeature<dyn AddMessage>,

    /// The optional send message feature.
    #[cfg(feature = "message-send")]
    pub send_message: BackendFeature<dyn SendMessage>,

    /// The optional peek messages feature.
    #[cfg(feature = "message-peek")]
    pub peek_messages: BackendFeature<dyn PeekMessages>,

    /// The optional get messages feature.
    #[cfg(feature = "message-get")]
    pub get_messages: BackendFeature<dyn GetMessages>,

    /// The optional copy messages feature.
    #[cfg(feature = "message-copy")]
    pub copy_messages: BackendFeature<dyn CopyMessages>,

    /// The optional move messages feature.
    #[cfg(feature = "message-move")]
    pub move_messages: BackendFeature<dyn MoveMessages>,

    /// The optional delete messages feature.
    #[cfg(feature = "message-delete")]
    pub delete_messages: BackendFeature<dyn DeleteMessages>,
}

impl<C: BackendContext> Backend<C> {
    /// Build a new backend from an account configuration and a
    /// context.
    pub fn new(account_config: Arc<AccountConfig>, context: C) -> Self {
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
    pub fn set_add_folder(&mut self, f: BackendFeature<dyn AddFolder>) {
        self.add_folder = f;
    }

    /// Set the list folders backend feature.
    #[cfg(feature = "folder-list")]
    pub fn set_list_folders(&mut self, f: BackendFeature<dyn ListFolders>) {
        self.list_folders = f;
    }
    /// Set the expunge folder backend feature.
    #[cfg(feature = "folder-expunge")]
    pub fn set_expunge_folder(&mut self, f: BackendFeature<dyn ExpungeFolder>) {
        self.expunge_folder = f;
    }

    /// Set the purge folder backend feature.
    #[cfg(feature = "folder-purge")]
    pub fn set_purge_folder(&mut self, f: BackendFeature<dyn PurgeFolder>) {
        self.purge_folder = f;
    }

    /// Set the delete folder backend feature.
    #[cfg(feature = "folder-delete")]
    pub fn set_delete_folder(&mut self, f: BackendFeature<dyn DeleteFolder>) {
        self.delete_folder = f;
    }

    /// Set the list envelopes backend feature.
    #[cfg(feature = "envelope-list")]
    pub fn set_list_envelopes(&mut self, f: BackendFeature<dyn ListEnvelopes>) {
        self.list_envelopes = f;
    }

    /// Set the watch envelopes backend feature.
    #[cfg(feature = "envelope-watch")]
    pub fn set_watch_envelopes(&mut self, f: BackendFeature<dyn WatchEnvelopes>) {
        self.watch_envelopes = f;
    }

    /// Set the get envelope backend feature.
    #[cfg(feature = "envelope-get")]
    pub fn set_get_envelope(&mut self, f: BackendFeature<dyn GetEnvelope>) {
        self.get_envelope = f;
    }

    /// Set the add flags backend feature.
    #[cfg(feature = "flag-add")]
    pub fn set_add_flags(&mut self, f: BackendFeature<dyn AddFlags>) {
        self.add_flags = f;
    }

    /// Set the set flags backend feature.
    #[cfg(feature = "flag-set")]
    pub fn set_set_flags(&mut self, f: BackendFeature<dyn SetFlags>) {
        self.set_flags = f;
    }

    /// Set the remove flags backend feature.
    #[cfg(feature = "flag-remove")]
    pub fn set_remove_flags(&mut self, f: BackendFeature<dyn RemoveFlags>) {
        self.remove_flags = f;
    }

    /// Set the add message backend feature.
    #[cfg(feature = "message-add")]
    pub fn set_add_message(&mut self, f: BackendFeature<dyn AddMessage>) {
        self.add_message = f;
    }

    /// Set the send message backend feature.
    #[cfg(feature = "message-send")]
    pub fn set_send_message(&mut self, f: BackendFeature<dyn SendMessage>) {
        self.send_message = f;
    }

    /// Set the peek messages backend feature.
    #[cfg(feature = "message-peek")]
    pub fn set_peek_messages(&mut self, f: BackendFeature<dyn PeekMessages>) {
        self.peek_messages = f;
    }

    /// Set the get messages backend feature.
    #[cfg(feature = "message-get")]
    pub fn set_get_messages(&mut self, f: BackendFeature<dyn GetMessages>) {
        self.get_messages = f;
    }

    /// Set the copy messages backend feature.
    #[cfg(feature = "message-copy")]
    pub fn set_copy_messages(&mut self, f: BackendFeature<dyn CopyMessages>) {
        self.copy_messages = f;
    }

    /// Set the move messages backend feature.
    #[cfg(feature = "message-move")]
    pub fn set_move_messages(&mut self, f: BackendFeature<dyn MoveMessages>) {
        self.move_messages = f;
    }

    /// Set the delete messages backend feature.
    #[cfg(feature = "message-delete")]
    pub fn set_delete_messages(&mut self, f: BackendFeature<dyn DeleteMessages>) {
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
    pub async fn send_message(&self, msg: &[u8]) -> Result<()> {
        self.send_message
            .as_ref()
            .ok_or(Error::SendMessageNotAvailableError)?
            .send_message(msg)
            .await?;

        #[cfg(feature = "message-add")]
        if self.account_config.should_save_copy_sent_message() {
            let folder = self.account_config.get_sent_folder_alias();
            log::debug!("saving copy of sent message to {folder}");
            self.add_message_with_flag(&folder, msg, Flag::Seen).await?;
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
        self.get_messages
            .as_ref()
            .ok_or(Error::GetMessagesNotAvailableError)?
            .get_messages(folder, id)
            .await
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
        self.delete_messages
            .as_ref()
            .ok_or(Error::DeleteMessagesNotAvailableError)?
            .delete_messages(folder, id)
            .await
    }
}
