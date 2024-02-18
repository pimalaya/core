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
//! ```rust,ignore
#![doc = include_str!("../../tests/dynamic_backend.rs")]
//! ```
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
//!
//! ```rust,ignore
#![doc = include_str!("../../tests/static_backend.rs")]
//! ```

pub mod context;
pub mod feature;
pub mod mapper;
pub mod pool;
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
    Result,
};

use self::{
    context::{BackendContext, BackendContextBuilder},
    feature::{AsyncTryIntoBackendFeatures, BackendFeature, BackendFeatureSource, BackendFeatures},
};

/// Errors related to backend.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot list folders: feature not available")]
    ListFoldersNotAvailableError,
}

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

    /// The optional add folder feature.
    pub list_folders: Option<BackendFeature<C, dyn ListFolders>>,
}

#[async_trait]
impl<C> ListFolders for Backend<C>
where
    C: BackendContext,
{
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

#[async_trait]
impl<CB> AsyncTryIntoBackendFeatures<Backend<CB::Context>> for BackendBuilder<CB>
where
    CB: BackendContextBuilder,
{
    async fn try_into_backend(self) -> Result<Backend<CB::Context>> {
        let list_folders = match self.list_folders {
            BackendFeatureSource::None => None,
            BackendFeatureSource::Context => self.ctx_builder.list_folders(),
            BackendFeatureSource::Backend(f) => Some(f),
        };

        Ok(Backend {
            account_config: self.account_config,
            context: Arc::new(self.ctx_builder.build().await?),
            list_folders,
        })
    }
}

/// Macro for defining [`BackendBuilder`] feature setters.
macro_rules! feature_setters {
    ($feat:ty) => {
        paste! {
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
    /// The backend context builder.
    ctx_builder: CB,

    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The list folders feature.
    pub list_folders: BackendFeatureSource<CB::Context, dyn ListFolders>,
}

impl<CB> BackendBuilder<CB>
where
    CB: BackendContextBuilder,
{
    /// Create a new backend builder using the given backend context
    /// builder.
    ///
    /// All features are taken from the context by default.
    pub fn new(account_config: Arc<AccountConfig>, ctx_builder: CB) -> Self {
        Self {
            account_config,
            ctx_builder,
            list_folders: BackendFeatureSource::Context,
        }
    }

    /// Disable all features for this backend builder.
    pub fn without_features(mut self) -> Self {
        self.set_list_folders(BackendFeatureSource::None);
        self
    }

    feature_setters!(ListFolders);

    /// Build the final backend.
    ///
    /// The backend instance should implement
    /// [`AsyncTryIntoBackendFeatures`].
    pub async fn build<B>(self) -> Result<B>
    where
        B: BackendFeatures,
        Self: AsyncTryIntoBackendFeatures<B>,
    {
        self.try_into_backend().await
    }
}
