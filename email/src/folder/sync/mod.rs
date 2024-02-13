//! Module dedicated to folders synchronization.
//!
//! This module contains everything you need to synchronize remote
//! folders with local ones.

pub mod cache;
mod hunk;
pub mod patch;
mod report;
pub mod worker;

use futures::{stream::FuturesUnordered, Future, StreamExt};
use log::debug;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, path::PathBuf, pin::Pin, sync::Arc};
use thiserror::Error;

use crate::{
    backend::{BackendBuilder, BackendContextBuilder},
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    sync::{pool::SyncPoolContext, SyncDestination, SyncEvent, SyncEventHandler},
    thread_pool::ThreadPool,
    Result,
};

use self::patch::build_patch;
#[doc(inline)]
pub use self::{
    cache::FolderSyncCache,
    hunk::{FolderName, FolderSyncCacheHunk, FolderSyncHunk, FoldersName},
    patch::{FolderSyncCachePatch, FolderSyncPatch, FolderSyncPatchManager, FolderSyncPatches},
    report::FolderSyncReport,
};

use super::Folder;

/// Errors related to synchronization.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get folder sync cache directory")]
    GetCacheDirectoryError,
}

/// The folder synchronization strategy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FolderSyncStrategy {
    /// Synchronizes all folders.
    #[default]
    All,

    /// Synchronizes only folders matching the given names.
    Include(HashSet<String>),

    /// Synchronizes all folders except the ones matching the given
    /// names.
    Exclude(HashSet<String>),
}

impl FolderSyncStrategy {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

pub type FolderSyncEventHandler =
    dyn Fn(FolderSyncEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync;

/// The backend synchronization progress event.
///
/// Represents all the events that can be triggered during the backend
/// synchronization process.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum FolderSyncEvent {
    ListedLeftCachedFolders(usize),
    ListedRightCachedFolders(usize),
    ListedLeftFolders(usize),
    ListedRightFolders(usize),
    ListedAllFolders,
    ProcessedFolderHunk(FolderSyncHunk),
}

impl FolderSyncEvent {
    pub async fn emit(&self, handler: &Option<Arc<FolderSyncEventHandler>>) {
        debug!("emitting folder sync event {self:?}");

        if let Some(handler) = handler.as_ref() {
            if let Err(err) = handler(self.clone()).await {
                debug!("error while emitting folder sync event, ignoring it");
                debug!("{err:?}");
            }
        }
    }
}

impl fmt::Display for FolderSyncEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FolderSyncEvent::*;

        match self {
            ListedLeftCachedFolders(n) => {
                write!(f, "Listed {n} left folders from cache")
            }
            ListedRightCachedFolders(n) => {
                write!(f, "Listed {n} right folders from cache")
            }
            ListedLeftFolders(n) => {
                write!(f, "Listed {n} left folders")
            }
            ListedRightFolders(n) => {
                write!(f, "Listed {n} right folders")
            }
            ListedAllFolders => {
                write!(f, "Listed all folders")
            }
            ProcessedFolderHunk(hunk) => {
                write!(f, "{hunk}")
            }
        }
    }
}

#[derive(Clone)]
pub struct FolderSyncBuilder<L, R>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    id: String,
    left_builder: BackendBuilder<L>,
    right_builder: BackendBuilder<R>,
    handler: Option<Arc<FolderSyncEventHandler>>,
    cache_dir: Option<PathBuf>,
}

impl<L, R> FolderSyncBuilder<L, R>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    pub fn new(left_builder: BackendBuilder<L>, right_builder: BackendBuilder<R>) -> Self {
        let id = left_builder.account_config.name.clone() + &right_builder.account_config.name;
        let id = format!("{:x}", md5::compute(id));

        Self {
            id,
            left_builder,
            right_builder,
            handler: None,
            cache_dir: None,
        }
    }

    pub fn set_some_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: Option<impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static>,
    ) {
        self.handler = match handler {
            Some(handler) => Some(Arc::new(move |evt| Box::pin(handler(evt)))),
            None => None,
        };
    }

    pub fn set_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static,
    ) {
        self.set_some_handler(Some(handler));
    }

    pub fn with_some_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: Option<impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static>,
    ) -> Self {
        self.set_some_handler(handler);
        self
    }

    pub fn with_some_atomic_handler_ref(
        mut self,
        handler: Option<Arc<FolderSyncEventHandler>>,
    ) -> Self {
        self.handler = handler;
        self
    }

    pub fn with_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static,
    ) -> Self {
        self.set_handler(handler);
        self
    }

    pub fn set_some_cache_dir(&mut self, dir: Option<impl Into<PathBuf>>) {
        self.cache_dir = dir.map(Into::into);
    }

    pub fn set_cache_dir(&mut self, dir: impl Into<PathBuf>) {
        self.set_some_cache_dir(Some(dir));
    }

    pub fn with_some_cache_dir(mut self, dir: Option<impl Into<PathBuf>>) -> Self {
        self.set_some_cache_dir(dir);
        self
    }

    pub fn with_cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.set_cache_dir(dir);
        self
    }

    pub fn find_default_cache_dir(&self) -> Option<PathBuf> {
        dirs::cache_dir().map(|dir| {
            dir.join("pimalaya")
                .join("email")
                .join("sync")
                .join(&self.id)
        })
    }

    pub fn find_cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir
            .as_ref()
            .cloned()
            .or_else(|| self.find_default_cache_dir())
    }

    pub fn get_cache_dir(&self) -> Result<PathBuf> {
        self.find_cache_dir()
            .ok_or(Error::GetCacheDirectoryError.into())
    }

    pub async fn sync(self) -> Result<FolderSyncReport> {
        let cache_dir = self.get_cache_dir()?;
        let left_config = self.left_builder.account_config.clone();
        let right_config = self.left_builder.account_config.clone();

        let handler = self.handler.clone();
        let root_dir = cache_dir.join(&left_config.name);
        let ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig { root_dir }));
        let left_cached_builder = BackendBuilder::new(left_config.clone(), ctx);
        let left_cached_builder_clone = left_cached_builder.clone();
        let left_folders_cached = tokio::spawn(async move {
            let folders = left_cached_builder_clone
                .build()
                .await?
                .list_folders()
                .await?;

            let names = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            FolderSyncEvent::ListedLeftCachedFolders(names.len())
                .emit(&handler)
                .await;

            Result::Ok(names)
        });

        let root_dir = cache_dir.join(&right_config.name);
        let handler = self.handler.clone();
        let ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig { root_dir }));
        let right_cached_builder = BackendBuilder::new(right_config.clone(), ctx);
        let right_cached_builder_clone = right_cached_builder.clone();
        let right_folders_cached = tokio::spawn(async move {
            let folders = right_cached_builder_clone
                .build()
                .await?
                .list_folders()
                .await?;

            let names = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            FolderSyncEvent::ListedRightCachedFolders(names.len())
                .emit(&handler)
                .await;

            Result::Ok(names)
        });

        let handler = self.handler.clone();
        let left_builder = self.left_builder.clone();
        let left_folders = tokio::spawn(async move {
            let folders = left_builder.build().await?.list_folders().await?;

            let names = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            FolderSyncEvent::ListedLeftFolders(names.len())
                .emit(&handler)
                .await;

            Result::Ok(names)
        });

        let handler = self.handler.clone();
        let right_builder = self.right_builder.clone();
        let right_folders = tokio::spawn(async move {
            let folders = right_builder.build().await?.list_folders().await?;

            let names = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            FolderSyncEvent::ListedRightFolders(names.len())
                .emit(&handler)
                .await;

            Result::Ok(names)
        });

        let (left_folders_cached, left_folders, right_folders_cached, right_folders) = tokio::try_join!(
            left_folders_cached,
            left_folders,
            right_folders_cached,
            right_folders,
        )?;

        FolderSyncEvent::ListedAllFolders.emit(&self.handler).await;

        let patches = build_patch(
            left_folders_cached?,
            left_folders?,
            right_folders_cached?,
            right_folders?,
        );

        let patch: FolderSyncPatch = patches.into_values().flatten().collect();

        let report = worker::process_patch(
            self.left_builder.clone(),
            left_cached_builder.clone(),
            self.right_builder.clone(),
            right_cached_builder.clone(),
            self.handler,
            patch,
            8,
        )
        .await;

        Ok(report)
    }
}

pub(crate) async fn sync<L, R>(
    pool: &ThreadPool<SyncPoolContext<L::Context, R::Context>>,
    handler: &Option<Arc<SyncEventHandler>>,
) -> Result<FolderSyncReport>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let mut report = FolderSyncReport::default();

    let left_cached_folders = pool.exec(|ctx| async move {
        let folders = ctx.left_cache.list_folders().await?;
        let names = HashSet::<String>::from_iter(
            folders
                .iter()
                .map(Folder::get_kind_or_name)
                .map(ToOwned::to_owned),
        );

        SyncEvent::ListedLeftCachedFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let left_folders = pool.exec(|ctx| async move {
        let folders = ctx.left.list_folders().await?;
        let names = HashSet::<String>::from_iter(
            folders
                .iter()
                .map(Folder::get_kind_or_name)
                .map(ToOwned::to_owned),
        );

        SyncEvent::ListedLeftFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let right_cached_folders = pool.exec(|ctx| async move {
        let folders = ctx.right_cache.list_folders().await?;
        let names = HashSet::<String>::from_iter(
            folders
                .iter()
                .map(Folder::get_kind_or_name)
                .map(ToOwned::to_owned),
        );

        SyncEvent::ListedRightCachedFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let right_folders = pool.exec(|ctx| async move {
        let folders = ctx.right.list_folders().await?;
        let names: HashSet<String> = HashSet::from_iter(
            folders
                .iter()
                .map(Folder::get_kind_or_name)
                .map(ToOwned::to_owned),
        );

        SyncEvent::ListedRightFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let (left_cached_folders, left_folders, right_cached_folders, right_folders) = tokio::try_join!(
        left_cached_folders,
        left_folders,
        right_cached_folders,
        right_folders
    )?;

    SyncEvent::ListedAllFolders.emit(&handler).await;

    let patch = build_patch(
        left_cached_folders,
        left_folders,
        right_cached_folders,
        right_folders,
    );

    let (folders, patch) = patch.into_iter().fold(
        (HashSet::default(), vec![]),
        |(mut folders, mut patch), (folder, hunks)| {
            folders.insert(folder);
            patch.extend(hunks);
            (folders, patch)
        },
    );

    report.names = folders;
    report.patch = FuturesUnordered::from_iter(patch.into_iter().map(|hunk| {
        pool.exec(move |ctx| {
            let hunk_clone = hunk.clone();
            let handler = ctx.handler.clone();
            let task = async move {
                match hunk_clone {
                    FolderSyncHunk::Cache(folder, SyncDestination::Left) => {
                        ctx.left_cache.add_folder(&folder).await
                    }
                    FolderSyncHunk::Create(folder, SyncDestination::Left) => {
                        ctx.left.add_folder(&folder).await
                    }
                    FolderSyncHunk::Cache(folder, SyncDestination::Right) => {
                        ctx.right_cache.add_folder(&folder).await
                    }
                    FolderSyncHunk::Create(folder, SyncDestination::Right) => {
                        ctx.right.add_folder(&folder).await
                    }
                    FolderSyncHunk::Uncache(folder, SyncDestination::Left) => {
                        ctx.left_cache.delete_folder(&folder).await
                    }
                    FolderSyncHunk::Delete(folder, SyncDestination::Left) => {
                        ctx.left.delete_folder(&folder).await
                    }
                    FolderSyncHunk::Uncache(folder, SyncDestination::Right) => {
                        ctx.right_cache.delete_folder(&folder).await
                    }
                    FolderSyncHunk::Delete(folder, SyncDestination::Right) => {
                        ctx.right.delete_folder(&folder).await
                    }
                }
            };

            async move {
                let output = task.await;

                SyncEvent::ProcessedFolderHunk(hunk.clone())
                    .emit(&handler)
                    .await;

                match output {
                    Ok(()) => (hunk, None),
                    Err(err) => (hunk, Some(err)),
                }
            }
        })
    }))
    .collect::<Vec<_>>()
    .await;

    Ok(report)
}

pub(crate) async fn expunge<L, R>(
    pool: &ThreadPool<SyncPoolContext<L::Context, R::Context>>,
    folders: &HashSet<String>,
) where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    FuturesUnordered::from_iter(folders.iter().map(|folder_ref| {
        let folder = folder_ref.clone();
        let left_cached_expunge =
            pool.exec(|ctx| async move { ctx.left_cache.expunge_folder(&folder).await });

        let folder = folder_ref.clone();
        let left_expunge = pool.exec(|ctx| async move { ctx.left.expunge_folder(&folder).await });

        let folder = folder_ref.clone();
        let right_cached_expunge =
            pool.exec(|ctx| async move { ctx.right_cache.expunge_folder(&folder).await });

        let folder = folder_ref.clone();
        let right_expunge = pool.exec(|ctx| async move { ctx.right.expunge_folder(&folder).await });

        async move {
            tokio::try_join!(
                left_cached_expunge,
                left_expunge,
                right_cached_expunge,
                right_expunge
            )
        }
    }))
    .filter_map(|res| async {
        match res {
            Ok(res) => Some(res),
            Err(err) => {
                debug!("cannot join tasks: {err}");
                None
            }
        }
    })
    .for_each(|_| async {})
    .await;
}
