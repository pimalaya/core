//! # Backend feature
//!
//! A [`BackendFeature`] is an action like adding folder, listing
//! envelopes or sending message. A feature needs a backend context to
//! be executed.

use std::sync::Arc;

use async_trait::async_trait;

use super::{context::BackendContext, AnyResult};
use crate::{
    account::config::HasAccountConfig,
    envelope::{get::GetEnvelope, list::ListEnvelopes},
    flag::{add::AddFlags, remove::RemoveFlags, set::SetFlags},
    folder::{
        add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders,
        purge::PurgeFolder,
    },
    message::{
        add::AddMessage, copy::CopyMessages, delete::DeleteMessages, get::GetMessages,
        peek::PeekMessages, r#move::MoveMessages, send::SendMessage,
    },
};

/// Backend builder feature for checking up configuration and context
/// integrity.
///
/// This feature is used to check the integrity of the context.
#[async_trait]
pub trait CheckUp: Send + Sync {
    /// Define how the no operation should be executed.
    async fn check_up(&self) -> AnyResult<()> {
        Ok(())
    }
}

/// The backend feature.
///
/// A backend feature is a function that takes a reference to a
/// backend context as parameter and returns a feature.
pub type BackendFeature<C, F> = Arc<dyn Fn(&C) -> Option<Box<F>> + Send + Sync>;

/// The backend feature source.
///
/// This enum is used by the backend builder to determine where a
/// specific backend feature should be taken from.
#[derive(Default)]
pub enum BackendFeatureSource<C: BackendContext, F: ?Sized> {
    /// The feature should be disabled.
    None,

    /// The feature should be taken from the
    /// [`super::BackendContextBuilder`].
    #[default]
    Context,

    /// The feature should be taken from the
    /// [`super::BackendBuilder`], using the given feature.
    Backend(BackendFeature<C, F>),
}

impl<C, F> Clone for BackendFeatureSource<C, F>
where
    C: BackendContext,
    F: ?Sized,
{
    fn clone(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Context => Self::Context,
            Self::Backend(f) => Self::Backend(f.clone()),
        }
    }
}

impl<C, F, T> From<T> for BackendFeatureSource<C, F>
where
    C: BackendContext,
    F: ?Sized,
    T: Fn(&C) -> Option<Box<F>> + Send + Sync + 'static,
{
    fn from(value: T) -> Self {
        Self::Backend(Arc::new(value))
    }
}

/// The backend features supertrait.
///
/// This trait is just an alias for all existing backend features.
pub trait BackendFeatures:
    HasAccountConfig
    + AddFolder
    + ListFolders
    + ExpungeFolder
    + PurgeFolder
    + DeleteFolder
    + GetEnvelope
    + ListEnvelopes
    + AddFlags
    + SetFlags
    + RemoveFlags
    + AddMessage
    + SendMessage
    + PeekMessages
    + GetMessages
    + CopyMessages
    + MoveMessages
    + DeleteMessages
{
}

/// Automatically implement [`BackendFeatures`] for structures
/// implementing all existing backend features.
impl<T> BackendFeatures for T where
    T: HasAccountConfig
        + AddFolder
        + ListFolders
        + ExpungeFolder
        + PurgeFolder
        + DeleteFolder
        + GetEnvelope
        + ListEnvelopes
        + AddFlags
        + SetFlags
        + RemoveFlags
        + AddMessage
        + SendMessage
        + PeekMessages
        + GetMessages
        + CopyMessages
        + MoveMessages
        + DeleteMessages
{
}

/// The backend implementation builder.
///
/// This trait defines how to build a backend implementation from a
/// [`BackendFeatures`] implementation.
#[async_trait]
pub trait AsyncTryIntoBackendFeatures<B>
where
    B: BackendFeatures,
{
    async fn try_into_backend(self) -> AnyResult<B>;
}
