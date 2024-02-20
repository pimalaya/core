use async_trait::async_trait;
use std::sync::Arc;

use crate::{
    backend::{
        context::{BackendContext, BackendContextBuilder},
        Backend, BackendBuilder,
    },
    folder::sync::config::FolderSyncStrategy,
    maildir::{MaildirContextBuilder, MaildirContextSync},
    thread_pool::{ThreadPool, ThreadPoolBuilder, ThreadPoolContext, ThreadPoolContextBuilder},
    Result,
};

use super::SyncEventHandler;

/// Create a new thread pool dedicated to synchronization.
pub async fn new<L, R>(
    left_cache_builder: BackendBuilder<MaildirContextBuilder>,
    left_builder: BackendBuilder<L>,
    right_cache_builder: BackendBuilder<MaildirContextBuilder>,
    right_builder: BackendBuilder<R>,
    handler: Option<Arc<SyncEventHandler>>,
    dry_run: bool,
    folder_filter: Option<FolderSyncStrategy>,
) -> Result<ThreadPool<SyncPoolContext<L::Context, R::Context>>>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let pool_ctx_builder = SyncPoolContextBuilder::new(
        left_cache_builder,
        left_builder,
        right_cache_builder,
        right_builder,
        handler,
        dry_run,
        folder_filter,
    );

    let pool_builder = ThreadPoolBuilder::new(pool_ctx_builder);

    let pool = pool_builder.build().await?;

    Ok(pool)
}

#[derive(Clone)]
pub struct SyncPoolContextBuilder<L, R>
where
    L: BackendContextBuilder,
    R: BackendContextBuilder,
{
    left_cache_builder: BackendBuilder<MaildirContextBuilder>,
    left_builder: BackendBuilder<L>,
    right_cache_builder: BackendBuilder<MaildirContextBuilder>,
    right_builder: BackendBuilder<R>,
    handler: Option<Arc<SyncEventHandler>>,
    dry_run: bool,
    folder_filter: Option<FolderSyncStrategy>,
}

impl<L, R> SyncPoolContextBuilder<L, R>
where
    L: BackendContextBuilder,
    R: BackendContextBuilder,
{
    pub fn new(
        left_cache_builder: BackendBuilder<MaildirContextBuilder>,
        left_builder: BackendBuilder<L>,
        right_cache_builder: BackendBuilder<MaildirContextBuilder>,
        right_builder: BackendBuilder<R>,
        handler: Option<Arc<SyncEventHandler>>,
        dry_run: bool,
        folder_filter: Option<FolderSyncStrategy>,
    ) -> Self {
        Self {
            left_cache_builder,
            left_builder,
            right_cache_builder,
            right_builder,
            handler,
            dry_run,
            folder_filter,
        }
    }
}

#[async_trait]
impl<L, R> ThreadPoolContextBuilder for SyncPoolContextBuilder<L, R>
where
    L: BackendContextBuilder,
    R: BackendContextBuilder,
{
    type Context = SyncPoolContext<L::Context, R::Context>;

    async fn build(self) -> Result<Self::Context> {
        let (left_cache, left, right_cache, right) = tokio::try_join!(
            self.left_cache_builder.build(),
            self.left_builder.build(),
            self.right_cache_builder.build(),
            self.right_builder.build(),
        )?;

        Ok(Self::Context {
            left_cache,
            left,
            right_cache,
            right,
            handler: self.handler,
            dry_run: self.dry_run,
            folder_filter: self.folder_filter,
        })
    }
}

pub struct SyncPoolContext<L: BackendContext, R: BackendContext> {
    pub left_cache: Backend<MaildirContextSync>,
    pub left: Backend<L>,
    pub right_cache: Backend<MaildirContextSync>,
    pub right: Backend<R>,
    pub handler: Option<Arc<SyncEventHandler>>,
    pub dry_run: bool,
    pub folder_filter: Option<FolderSyncStrategy>,
}

impl<L: BackendContext, R: BackendContext> SyncPoolContext<L, R> {
    pub fn matches_folder_filter(&self, folder: &str) -> bool {
        self.folder_filter
            .clone()
            .or_else(|| {
                self.right
                    .account_config
                    .folder
                    .as_ref()
                    .and_then(|c| c.sync.as_ref())
                    .map(|c| c.filter.clone())
            })
            .unwrap_or_default()
            .matches(folder)
    }
}

impl<L: BackendContext, R: BackendContext> ThreadPoolContext for SyncPoolContext<L, R> {
    //
}
