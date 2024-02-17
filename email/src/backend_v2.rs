//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the
//! [AccountConfiguration](crate::account::config::AccountConfig).

pub mod macros {
    pub use email_macros::BackendContextV2;
}

use async_trait::async_trait;
use paste::paste;
use std::sync::Arc;
use thiserror::Error;

use crate::{
    account::config::AccountConfig,
    folder::{list::ListFolders, Folders},
    thread_pool::{ThreadPool, ThreadPoolBuilder, ThreadPoolContext, ThreadPoolContextBuilder},
    Result,
};

/// Errors related to backend.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot list folders: feature not available")]
    ListFoldersNotAvailableError,
}

pub struct BackendBuilder<CB: BackendContextBuilder> {
    /// The backend context builder.
    ctx_builder: CB,

    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    pub list_folders: BackendFeatureSource<CB::Context, dyn ListFolders>,
}

macro_rules! backend_builder_feature {
    ($feat:ty) => {
        paste! {
            pub fn [<set_ $feat:snake>](
                &mut self,
                f: impl Into<BackendFeatureSource<CB::Context, dyn $feat>>,
            ) {
                self.[<$feat:snake>] = f.into();
            }

            pub fn [<with_ $feat:snake>](
                mut self,
                f: impl Into<BackendFeatureSource<CB::Context, dyn $feat>>,
            ) -> Self {
                self.[<set_ $feat:snake>](f);
                self
            }

            pub fn [<without_ $feat:snake>](mut self) -> Self {
                self.[<set_ $feat:snake>](BackendFeatureSource::None);
                self
            }

            pub fn [<with_context_ $feat:snake>](mut self) -> Self {
                self.[<set_ $feat:snake>](BackendFeatureSource::Context);
                self
            }
        }
    };
}

#[async_trait]
pub trait BuildBackend<B: Backend> {
    async fn build_backend(self) -> Result<B>;
}

impl<CB: BackendContextBuilder> BackendBuilder<CB> {
    /// Build a new backend builder using the given backend context
    /// builder.
    ///
    /// All features are disabled by default.
    pub fn new(account_config: Arc<AccountConfig>, ctx_builder: CB) -> Self {
        Self {
            account_config,
            ctx_builder,
            list_folders: BackendFeatureSource::Context,
        }
    }

    pub fn without_features(mut self) -> Self {
        self.set_list_folders(BackendFeatureSource::None);
        self
    }

    backend_builder_feature!(ListFolders);

    pub async fn build<B>(self) -> Result<B>
    where
        B: Backend,
        Self: BuildBackend<B>,
    {
        self.build_backend().await
    }
}

pub trait Backend
where
    Self: ListFolders,
{
    type Context: BackendContext;
}

pub struct BackendHandler<C: BackendContext> {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The backend context.
    pub context: Arc<C>,

    /// The optional add folder feature.
    pub list_folders: Option<BackendFeature<C, dyn ListFolders>>,
}

#[async_trait]
impl<C: BackendContext> ListFolders for BackendHandler<C> {
    async fn list_folders(&self) -> Result<Folders> {
        let feature = self
            .list_folders
            .as_ref()
            .ok_or(Error::ListFoldersNotAvailableError)?;

        feature(&self.context)
            .ok_or(Error::ListFoldersNotAvailableError)?
            .list_folders()
            .await
    }
}

impl<C: BackendContext> Backend for BackendHandler<C> {
    type Context = C;
}

#[async_trait]
impl<CB: BackendContextBuilder> BuildBackend<BackendHandler<CB::Context>> for BackendBuilder<CB> {
    async fn build_backend(self) -> Result<BackendHandler<CB::Context>> {
        let list_folders = match self.list_folders {
            BackendFeatureSource::None => None,
            BackendFeatureSource::Context => self.ctx_builder.list_folders(),
            BackendFeatureSource::Backend(f) => Some(f),
        };

        Ok(BackendHandler {
            account_config: self.account_config,
            context: Arc::new(self.ctx_builder.build().await?),
            list_folders,
        })
    }
}

pub struct BackendPool<C: BackendContext> {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The backend context.
    pub pool: ThreadPool<C>,

    /// The optional add folder feature.
    pub list_folders: Option<BackendFeature<C, dyn ListFolders>>,
}

#[async_trait]
impl<C: BackendContext + 'static> ListFolders for BackendPool<C> {
    async fn list_folders(&self) -> Result<Folders> {
        let feature = self
            .list_folders
            .clone()
            .ok_or(Error::ListFoldersNotAvailableError)?;

        self.pool
            .exec(|ctx| async move {
                feature(&ctx)
                    .ok_or(Error::ListFoldersNotAvailableError)?
                    .list_folders()
                    .await
            })
            .await
    }
}

impl<C: BackendContext + 'static> Backend for BackendPool<C> {
    type Context = C;
}

#[async_trait]
impl<CB: BackendContextBuilder + 'static> BuildBackend<BackendPool<CB::Context>>
    for BackendBuilder<CB>
{
    async fn build_backend(self) -> Result<BackendPool<CB::Context>> {
        let list_folders = match self.list_folders {
            BackendFeatureSource::None => None,
            BackendFeatureSource::Context => self.ctx_builder.list_folders(),
            BackendFeatureSource::Backend(f) => Some(f),
        };

        Ok(BackendPool {
            account_config: self.account_config.clone(),
            pool: ThreadPoolBuilder::new(self.ctx_builder).build().await?,
            list_folders,
        })
    }
}

#[derive(Clone, Default)]
pub enum BackendFeatureSource<C: BackendContext, F: ?Sized> {
    /// The feature should be initialized in [`Backend`] to `None`
    None,
    /// The feature should be initialized from the [`BackendContext`].
    /// If the context doesn't support this feature it will be initialized to `None`.
    #[default]
    Context,
    /// Use this given [`BackendFeatureBuilder`] to initialize the feature in [`Backend`].
    /// If this is a `BackendFeatureBuilder::none()`, it will try to initialize this feature
    /// from the context (as if [`FeatureConfiguration::Default`] was used).
    Backend(BackendFeature<C, F>),
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

/// The backend feature builder.
///
/// A feature builder is a function that takes an atomic reference to
/// a context as parameter and returns a backend feature.
pub type BackendFeature<C, F> = Arc<dyn Fn(&C) -> Option<Box<F>> + Send + Sync>;

macro_rules! backend_context_builder_feature {
    ($feat:ty) => {
        paste! {
            fn [<$feat:snake>](
                &self
            ) -> Option<BackendFeature<Self::Context, dyn $feat>> {
                None
            }
        }
    };
}

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

    backend_context_builder_feature!(ListFolders);

    /// Build the final context.
    async fn build(self) -> Result<Self::Context>;
}

#[async_trait]
impl<T: BackendContextBuilder> ThreadPoolContextBuilder for T {
    type Context = T::Context;

    async fn build(self) -> Result<Self::Context> {
        BackendContextBuilder::build(self).await
    }
}

/// The backend context trait.
///
/// This is just a marker for other traits. Every backend context
/// needs to implement this trait manually or to derive
/// [`email_macros::BackendContext`].
pub trait BackendContext: Send + Sync {
    //
}

impl<T: BackendContext> ThreadPoolContext for T {}

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

macro_rules! map_feature_from {
    ($feat:ty) => {
        paste! {
            fn [<$feat:snake _from>] (
                &self,
                cb: Option<&CB>,
            ) -> Option<BackendFeature<Self::Context, dyn $feat>> {
               self.map_feature(cb.and_then(|cb| cb.[<$feat:snake>]()))
            }
        }
    };
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
///
pub trait MapBackendFeature<CB>
where
    Self: BackendContextBuilder,
    Self::Context: FindBackendSubcontext<CB::Context> + 'static,
    CB: BackendContextBuilder,
    CB::Context: BackendContext + 'static,
{
    fn map_feature<T: ?Sized + 'static>(
        &self,
        f: Option<BackendFeature<CB::Context, T>>,
    ) -> Option<BackendFeature<Self::Context, T>> {
        let f = f?;
        Some(Arc::new(move |ctx| f(ctx.find_subcontext()?)))
    }

    map_feature_from!(ListFolders);
}

/// Generic implementation for the backend context builder with a
/// context implementing [`FindBackendSubcontext`].
impl<CB1, CB2> MapBackendFeature<CB2> for CB1
where
    CB1: BackendContextBuilder,
    CB1::Context: FindBackendSubcontext<CB2::Context> + 'static,
    CB2: BackendContextBuilder,
    CB2::Context: BackendContext + 'static,
{
    //
}
