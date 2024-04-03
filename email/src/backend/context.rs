//! # Backend context
//!
//! The [`BackendContext`] is usually used for storing clients or
//! sessions (structures than cannot be cloned or sync). The
//! [`BackendContextBuilder`] gives instructions on how to build such
//! context. It is used by the backend builder.

use async_trait::async_trait;
use paste::paste;

use crate::{
    envelope::{get::GetEnvelope, list::ListEnvelopes, watch::WatchEnvelopes},
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder,
    },
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        peek::PeekMessages, r#move::MoveMessages, remove::RemoveMessages, send::SendMessage,
    },
};

use super::feature::{BackendFeature, CheckUp};

/// The backend context.
///
/// This is just a marker for other backend traits. Every backend
/// context needs to implement this trait manually or to derive
/// [`crate::backend_v2::macros::BackendContextV2`].
pub trait BackendContext: Send + Sync {}

/// Macro for defining [`BackendContextBuilder`] features.
macro_rules! feature {
    ($feat:ty) => {
        paste! {
            /// Define the given backend feature.
            fn [<$feat:snake>](&self) -> Option<BackendFeature<Self::Context, dyn $feat>> {
                None
            }
        }
    };
}

/// The backend context builder.
///
/// This trait defines how a context should be built. It also defines
/// default backend features implemented by the context itself.
#[async_trait]
pub trait BackendContextBuilder: Clone + Send + Sync {
    /// The type of the context being built by this builder.
    type Context: BackendContext;

    feature!(CheckUp);

    feature!(AddFolder);
    feature!(ListFolders);
    feature!(ExpungeFolder);
    feature!(PurgeFolder);
    feature!(DeleteFolder);
    feature!(GetEnvelope);
    feature!(ListEnvelopes);
    feature!(WatchEnvelopes);
    feature!(AddFlags);
    feature!(SetFlags);
    feature!(RemoveFlags);
    feature!(AddMessage);
    feature!(SendMessage);
    feature!(PeekMessages);
    feature!(GetMessages);
    feature!(CopyMessages);
    feature!(MoveMessages);
    feature!(DeleteMessages);
    feature!(RemoveMessages);

    /// Build the final context used by the backend.
    async fn build(self) -> crate::Result<Self::Context>;
}
