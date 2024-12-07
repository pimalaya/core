//! # Backend feature mapper
//!
//! This module contains [`BackendContextBuilder`] helpers to map
//! features from a subcontext B to a context A.

use std::sync::Arc;

use paste::paste;

use super::{
    context::{BackendContext, BackendContextBuilder},
    feature::{BackendFeature, CheckUp},
};
#[cfg(feature = "thread")]
use crate::envelope::thread::ThreadEnvelopes;
#[cfg(feature = "watch")]
use crate::envelope::watch::WatchEnvelopes;
use crate::{
    envelope::{get::GetEnvelope, list::ListEnvelopes},
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

/// Macro for defining some [`BackendContextBuilder`] feature mapper.
macro_rules! some_feature_mapper {
    ($feat:ty) => {
        paste! {
            fn [<$feat:snake _with_some>](
                &self,
                cb: &Option<CB>,
            ) -> Option<BackendFeature<Self::Context, dyn $feat>> {
                let cb = cb.as_ref()?;
                self.map_feature(cb.[<$feat:snake>]())
            }
        }
    };
}

/// Map a backend feature from subcontext B to context A.
///
/// This is useful when you have a context composed of multiple
/// subcontexts. It prevents you to manually map the feature.
///
/// See a usage example at `../../tests/dynamic_backend.rs`.
pub trait SomeBackendContextBuilderMapper<CB>
where
    Self: BackendContextBuilder,
    Self::Context: AsRef<Option<CB::Context>> + 'static,
    CB: BackendContextBuilder,
    CB::Context: BackendContext + 'static,
{
    fn map_feature<T: ?Sized + 'static>(
        &self,
        f: Option<BackendFeature<CB::Context, T>>,
    ) -> Option<BackendFeature<Self::Context, T>> {
        let f = f?;
        Some(Arc::new(move |ctx| f(ctx.as_ref().as_ref()?)))
    }

    some_feature_mapper!(CheckUp);

    some_feature_mapper!(AddFolder);
    some_feature_mapper!(ListFolders);
    some_feature_mapper!(ExpungeFolder);
    some_feature_mapper!(PurgeFolder);
    some_feature_mapper!(DeleteFolder);
    some_feature_mapper!(GetEnvelope);
    some_feature_mapper!(ListEnvelopes);
    #[cfg(feature = "thread")]
    some_feature_mapper!(ThreadEnvelopes);
    #[cfg(feature = "watch")]
    some_feature_mapper!(WatchEnvelopes);
    some_feature_mapper!(AddFlags);
    some_feature_mapper!(SetFlags);
    some_feature_mapper!(RemoveFlags);
    some_feature_mapper!(AddMessage);
    some_feature_mapper!(SendMessage);
    some_feature_mapper!(PeekMessages);
    some_feature_mapper!(GetMessages);
    some_feature_mapper!(CopyMessages);
    some_feature_mapper!(MoveMessages);
    some_feature_mapper!(DeleteMessages);
    some_feature_mapper!(RemoveMessages);
}

/// Automatically implement [`SomeBackendContextBuilderMapper`].
impl<CB1, CB2> SomeBackendContextBuilderMapper<CB2> for CB1
where
    CB1: BackendContextBuilder,
    CB1::Context: AsRef<Option<CB2::Context>> + 'static,
    CB2: BackendContextBuilder,
    CB2::Context: BackendContext + 'static,
{
}

/// Macro for defining [`BackendContextBuilder`] feature mapper.
macro_rules! feature_mapper {
    ($feat:ty) => {
        paste! {
            fn [<$feat:snake _with>] (
                &self,
                cb: &CB,
            ) -> Option<BackendFeature<Self::Context, dyn $feat>> {
               self.map_feature(cb.[<$feat:snake>]())
            }
        }
    };
}

/// Same as [`SomeBackendContextBuilderMapper`] but without Option.
pub trait BackendContextBuilderMapper<CB>
where
    Self: BackendContextBuilder,
    Self::Context: AsRef<CB::Context> + 'static,
    CB: BackendContextBuilder,
    CB::Context: BackendContext + 'static,
{
    fn map_feature<T: ?Sized + 'static>(
        &self,
        f: Option<BackendFeature<CB::Context, T>>,
    ) -> Option<BackendFeature<Self::Context, T>> {
        let f = f?;
        Some(Arc::new(move |ctx| f(ctx.as_ref())))
    }

    feature_mapper!(AddFolder);
    feature_mapper!(ListFolders);
    feature_mapper!(ExpungeFolder);
    feature_mapper!(PurgeFolder);
    feature_mapper!(DeleteFolder);
    feature_mapper!(GetEnvelope);
    feature_mapper!(ListEnvelopes);
    #[cfg(feature = "thread")]
    feature_mapper!(ThreadEnvelopes);
    #[cfg(feature = "watch")]
    feature_mapper!(WatchEnvelopes);
    feature_mapper!(AddFlags);
    feature_mapper!(SetFlags);
    feature_mapper!(RemoveFlags);
    feature_mapper!(AddMessage);
    feature_mapper!(SendMessage);
    feature_mapper!(PeekMessages);
    feature_mapper!(GetMessages);
    feature_mapper!(CopyMessages);
    feature_mapper!(MoveMessages);
    feature_mapper!(DeleteMessages);
    feature_mapper!(RemoveMessages);
}

/// Automatically implement [`BackendContextBuilderMapper`].
impl<CB1, CB2> BackendContextBuilderMapper<CB2> for CB1
where
    CB1: BackendContextBuilder,
    CB1::Context: AsRef<CB2::Context> + 'static,
    CB2: BackendContextBuilder,
    CB2::Context: BackendContext + 'static,
{
}
