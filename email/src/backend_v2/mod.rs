//! Module dedicated to backend management.
//!
//! The core concept of this module is the [`Backend`] trait, which is
//! an abstraction over emails manipulation.
//!
//! Then you have the [`BackendConfig`] which represents the
//! backend-specific configuration, mostly used by the
//! [AccountConfiguration](crate::account::config::AccountConfig).

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

macro_rules! feature_setters {
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
pub struct BackendBuilder<CB: BackendContextBuilder> {
    /// The backend context builder.
    ctx_builder: CB,

    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    pub list_folders: BackendFeatureSource<CB::Context, dyn ListFolders>,
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

    feature_setters!(ListFolders);

    pub async fn build<B>(self) -> Result<B>
    where
        B: BackendFeatures,
        Self: AsyncTryIntoBackendFeatures<B>,
    {
        self.try_into_backend().await
    }
}

pub struct Backend<C: BackendContext> {
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The backend context.
    pub context: Arc<C>,

    /// The optional add folder feature.
    pub list_folders: Option<BackendFeature<C, dyn ListFolders>>,
}

#[async_trait]
impl<C: BackendContext> ListFolders for Backend<C> {
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
