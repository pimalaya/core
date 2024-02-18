use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    account::config::AccountConfig,
    folder::{list::ListFolders, Folders},
    thread_pool::{ThreadPool, ThreadPoolBuilder, ThreadPoolContext, ThreadPoolContextBuilder},
    Result,
};

use super::{
    context::{BackendContext, BackendContextBuilder},
    feature::{BackendFeature, BackendFeatureSource},
    AsyncTryIntoBackendFeatures, BackendBuilder, Error,
};

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

#[async_trait]
impl<CB> AsyncTryIntoBackendFeatures<BackendPool<CB::Context>> for BackendBuilder<CB>
where
    CB: BackendContextBuilder + 'static,
{
    async fn try_into_backend(self) -> Result<BackendPool<CB::Context>> {
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

#[async_trait]
impl<T: BackendContextBuilder> ThreadPoolContextBuilder for T {
    type Context = T::Context;

    async fn build(self) -> Result<Self::Context> {
        BackendContextBuilder::build(self).await
    }
}

impl<T: BackendContext> ThreadPoolContext for T {}
