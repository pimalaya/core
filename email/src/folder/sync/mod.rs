//! # Folder synchronization
//!
//! This module contains everything you need to synchronize remote
//! folders with local ones.

pub mod config;
pub mod hunk;
pub mod patch;
pub mod report;

use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, trace};
use std::{collections::HashSet, sync::Arc};
use tokio::task::JoinHandle;

use crate::{
    backend::context::BackendContextBuilder,
    sync::{pool::SyncPoolContext, SyncDestination, SyncEvent},
    thread_pool::ThreadPool,
};

use super::{
    add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders, Folder,
};

use self::{hunk::FolderSyncHunk, report::FolderSyncReport};

pub(crate) async fn sync<L, R>(
    pool: Arc<ThreadPool<SyncPoolContext<L::Context, R::Context>>>,
) -> crate::Result<FolderSyncReport>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let mut report = FolderSyncReport::default();

    let pool_ref = pool.clone();
    let left_cached_folders: JoinHandle<crate::Result<HashSet<String>>> =
        tokio::spawn(async move {
            pool_ref
                .exec(|ctx| {
                    async move {
                        let folders = ctx.left_cache.list_folders().await?;
                        let names = HashSet::<String>::from_iter(
                            folders
                                .iter()
                                .map(Folder::get_kind_or_name)
                                // TODO: instead of fetching all the folders then
                                // filtering them here, it could be better to filter
                                // them at the source directly, which implies to add a
                                // new backend fn called `search_folders` and to set
                                // up a common search API across backends.
                                .filter_map(|folder| {
                                    if ctx.matches_folder_filter(folder) {
                                        Some(folder.to_owned())
                                    } else {
                                        None
                                    }
                                }),
                        );

                        SyncEvent::ListedLeftCachedFolders(names.len())
                            .emit(&ctx.handler)
                            .await;

                        Ok(names)
                    }
                })
                .await
        });

    let pool_ref = pool.clone();
    let left_folders: JoinHandle<crate::Result<HashSet<String>>> = tokio::spawn(async move {
        pool_ref
            .exec(|ctx| async move {
                let folders = ctx.left.list_folders().await?;
                let names = HashSet::<String>::from_iter(
                    folders
                        .iter()
                        .map(Folder::get_kind_or_name)
                        // TODO: instead of fetching all the folders then
                        // filtering them here, it could be better to filter
                        // them at the source directly, which implies to add a
                        // new backend fn called `search_folders` and to set
                        // up a common search API across backends.
                        .filter_map(|folder| {
                            if ctx.matches_folder_filter(folder) {
                                Some(folder.to_owned())
                            } else {
                                None
                            }
                        }),
                );

                SyncEvent::ListedLeftFolders(names.len())
                    .emit(&ctx.handler)
                    .await;

                Ok(names)
            })
            .await
    });

    let pool_ref = pool.clone();
    let right_cached_folders: JoinHandle<crate::Result<HashSet<String>>> =
        tokio::spawn(async move {
            pool_ref
                .exec(|ctx| async move {
                    let folders = ctx.right_cache.list_folders().await?;
                    let names = HashSet::<String>::from_iter(
                        folders
                            .iter()
                            .map(Folder::get_kind_or_name)
                            // TODO: instead of fetching all the folders then
                            // filtering them here, it could be better to filter
                            // them at the source directly, which implies to add a
                            // new backend fn called `search_folders` and to set
                            // up a common search API across backends.
                            .filter_map(|folder| {
                                if ctx.matches_folder_filter(folder) {
                                    Some(folder.to_owned())
                                } else {
                                    None
                                }
                            }),
                    );

                    SyncEvent::ListedRightCachedFolders(names.len())
                        .emit(&ctx.handler)
                        .await;

                    Ok(names)
                })
                .await
        });

    let pool_ref = pool.clone();
    let right_folders: JoinHandle<crate::Result<HashSet<String>>> = tokio::spawn(async move {
        pool_ref
            .exec(|ctx| async move {
                let folders = ctx.right.list_folders().await?;
                let names: HashSet<String> = HashSet::from_iter(
                    folders
                        .iter()
                        .map(Folder::get_kind_or_name)
                        // TODO: instead of fetching all the folders then
                        // filtering them here, it could be better to filter
                        // them at the source directly, which implies to add a
                        // new backend fn called `search_folders` and to set
                        // up a common search API across backends.
                        .filter_map(|folder| {
                            if ctx.matches_folder_filter(folder) {
                                Some(folder.to_owned())
                            } else {
                                None
                            }
                        }),
                );

                SyncEvent::ListedRightFolders(names.len())
                    .emit(&ctx.handler)
                    .await;

                Ok(names)
            })
            .await
    });

    let (left_cached_folders, left_folders, right_cached_folders, right_folders) = tokio::try_join!(
        left_cached_folders,
        left_folders,
        right_cached_folders,
        right_folders
    )?;

    pool.exec(|ctx| async move { SyncEvent::ListedAllFolders.emit(&ctx.handler).await })
        .await;

    let patch = patch::build(
        left_cached_folders?,
        left_folders?,
        right_cached_folders?,
        right_folders?,
    );

    let patch_clone = patch.clone();
    pool.exec(move |ctx| async move {
        SyncEvent::GeneratedFolderPatch(patch_clone)
            .emit(&ctx.handler)
            .await
    })
    .await;

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
        let pool = pool.clone();
        tokio::spawn(async move {
            pool.exec(|ctx| {
                let hunk_clone = hunk.clone();
                let handler = ctx.handler.clone();
                let task = async move {
                    if ctx.dry_run {
                        return Ok(());
                    }

                    match hunk_clone {
                        FolderSyncHunk::Cache(folder, SyncDestination::Left) => {
                            if ctx.left_cache.account_config.can_sync_create_folder() {
                                ctx.left_cache.add_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Create(folder, SyncDestination::Left) => {
                            if ctx.left.account_config.can_sync_create_folder() {
                                ctx.left.add_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Cache(folder, SyncDestination::Right) => {
                            if ctx.right_cache.account_config.can_sync_create_folder() {
                                ctx.right_cache.add_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Create(folder, SyncDestination::Right) => {
                            if ctx.right.account_config.can_sync_create_folder() {
                                ctx.right.add_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Uncache(folder, SyncDestination::Left) => {
                            if ctx.left_cache.account_config.can_sync_delete_folder() {
                                ctx.left_cache.delete_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Delete(folder, SyncDestination::Left) => {
                            if ctx.left.account_config.can_sync_delete_folder() {
                                ctx.left.delete_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Uncache(folder, SyncDestination::Right) => {
                            if ctx.right_cache.account_config.can_sync_delete_folder() {
                                ctx.right_cache.delete_folder(&folder).await?;
                            }
                        }
                        FolderSyncHunk::Delete(folder, SyncDestination::Right) => {
                            if ctx.right.account_config.can_sync_delete_folder() {
                                ctx.right.delete_folder(&folder).await?;
                            }
                        }
                    };

                    Ok(())
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
            .await
        })
    }))
    .filter_map(|hunk| async {
        match hunk {
            Ok(hunk) => Some(hunk),
            Err(err) => {
                debug!("cannot process folder hunk: {err}");
                trace!("{err:?}");
                None
            }
        }
    })
    .collect::<Vec<_>>()
    .await;

    pool.exec(|ctx| async move { SyncEvent::ProcessedAllFolderHunks.emit(&ctx.handler).await })
        .await;

    Ok(report)
}

pub(crate) async fn expunge<L, R>(
    pool: Arc<ThreadPool<SyncPoolContext<L::Context, R::Context>>>,
    folders: &HashSet<String>,
) where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let pool_clone = pool.clone();
    FuturesUnordered::from_iter(folders.iter().map(|folder_ref| {
        let folder = folder_ref.clone();
        let left_cached_expunge = pool_clone.exec(|ctx| async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.left_cache.expunge_folder(&folder).await
            }
        });

        let folder = folder_ref.clone();
        let left_expunge = pool_clone.exec(|ctx| async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.left.expunge_folder(&folder).await
            }
        });

        let folder = folder_ref.clone();
        let right_cached_expunge = pool_clone.exec(|ctx| async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.right_cache.expunge_folder(&folder).await
            }
        });

        let folder = folder_ref.clone();
        let right_expunge = pool_clone.exec(|ctx| async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.right.expunge_folder(&folder).await
            }
        });

        async {
            tokio::try_join!(
                left_cached_expunge,
                left_expunge,
                right_cached_expunge,
                right_expunge
            )
        }
    }))
    .for_each(|task| async {
        if let Err(err) = task {
            debug!("cannot expunge folders: {err}");
            trace!("{err:?}");
        }
    })
    .await;

    pool.exec(|ctx| async move { SyncEvent::ExpungedAllFolders.emit(&ctx.handler).await })
        .await;
}
