//! # Backend feature mapper
//!
//! This module contains [`BackendContextBuilder`] helpers to map
//! features from a subcontext B to a context A.

use paste::paste;
use std::sync::Arc;

use crate::folder::list::ListFolders;

use super::{
    context::{BackendContext, BackendContextBuilder},
    feature::BackendFeature,
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

// TODO: reword
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
///     async fn build(self, account_config: Arc<AccountConfig>) -> Result<Self::Context> {
///         let imap = match self.imap {
///             Some(imap) => Some(imap.build(account_config.clone()).await?),
///             None => None,
///         };
///
///         let smtp = match self.smtp {
///             Some(smtp) => Some(smtp.build(account_config).await?),
///             None => None,
///         };
///
///         Ok(MyContext { imap, smtp })
///     }
/// }
/// ```
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

    some_feature_mapper!(ListFolders);
}

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

    feature_mapper!(ListFolders);
}

impl<CB1, CB2> BackendContextBuilderMapper<CB2> for CB1
where
    CB1: BackendContextBuilder,
    CB1::Context: AsRef<CB2::Context> + 'static,
    CB2: BackendContextBuilder,
    CB2::Context: BackendContext + 'static,
{
}
