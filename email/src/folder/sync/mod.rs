//! # Folder synchronization
//!
//! This module contains everything you need to synchronize remote
//! folders with local ones.

pub mod config;
pub mod hunk;
pub mod patch;
pub mod report;

use std::{collections::HashSet, sync::Arc};

use futures::{stream::FuturesUnordered, StreamExt};
use tracing::{debug, trace};

use self::{hunk::FolderSyncHunk, report::FolderSyncReport};
use super::{
    add::AddFolder, delete::DeleteFolder, expunge::ExpungeFolder, list::ListFolders, Folder,
};
#[doc(inline)]
pub use super::{Error, Result};
use crate::{
    backend::context::BackendContextBuilder,
    sync::{pool::SyncPoolContext, SyncDestination, SyncEvent},
};

pub(crate) async fn sync<L, R>(
    ctx_ref: Arc<SyncPoolContext<L::Context, R::Context>>,
) -> Result<FolderSyncReport>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let mut report = FolderSyncReport::default();

    let ctx = ctx_ref.clone();
    let left_cached_folders = tokio::spawn(async move {
        let folders = ctx
            .left_cache
            .list_folders()
            .await
            .map_err(Error::ListLeftFoldersCachedError)?;
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
                    if ctx.folder_filters.matches(folder) {
                        Some(folder.to_owned())
                    } else {
                        None
                    }
                }),
        );

        SyncEvent::ListedLeftCachedFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let ctx = ctx_ref.clone();
    let left_folders = tokio::spawn(async move {
        let folders = ctx
            .left
            .list_folders()
            .await
            .map_err(Error::ListLeftFoldersError)?;
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
                    if ctx.folder_filters.matches(folder) {
                        Some(folder.to_owned())
                    } else {
                        None
                    }
                }),
        );

        SyncEvent::ListedLeftFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let ctx = ctx_ref.clone();
    let right_cached_folders = tokio::spawn(async move {
        let folders = ctx
            .right_cache
            .list_folders()
            .await
            .map_err(Error::ListRightFoldersCachedError)?;
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
                    if ctx.folder_filters.matches(folder) {
                        Some(folder.to_owned())
                    } else {
                        None
                    }
                }),
        );

        SyncEvent::ListedRightCachedFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let ctx = ctx_ref.clone();
    let right_folders = tokio::spawn(async move {
        let folders = ctx
            .right
            .list_folders()
            .await
            .map_err(Error::ListRightFoldersError)?;
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
                    if ctx.folder_filters.matches(folder) {
                        Some(folder.to_owned())
                    } else {
                        None
                    }
                }),
        );

        SyncEvent::ListedRightFolders(names.len())
            .emit(&ctx.handler)
            .await;

        Result::Ok(names)
    });

    let (left_cached_folders, left_folders, right_cached_folders, right_folders) =
        tokio::try_join!(
            left_cached_folders,
            left_folders,
            right_cached_folders,
            right_folders
        )
        .map_err(Error::FolderTasksFailed)?;

    SyncEvent::ListedAllFolders.emit(&ctx_ref.handler).await;

    let mut patch = patch::build(
        left_cached_folders?,
        left_folders?,
        right_cached_folders?,
        right_folders?,
    );

    ctx_ref.apply_folder_permissions(&mut patch);

    SyncEvent::GeneratedFolderPatch(patch.clone())
        .emit(&ctx_ref.handler)
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
        let ctx = ctx_ref.clone();
        tokio::spawn(async move {
            let hunk_clone = hunk.clone();
            let handler = ctx.handler.clone();
            let task = async move {
                if ctx.dry_run {
                    return Ok(());
                }

                match hunk_clone {
                    FolderSyncHunk::Cache(folder, SyncDestination::Left) => {
                        ctx.left_cache.add_folder(&folder).await?;
                    }
                    FolderSyncHunk::Create(folder, SyncDestination::Left) => {
                        ctx.left.add_folder(&folder).await?;
                    }
                    FolderSyncHunk::Cache(folder, SyncDestination::Right) => {
                        ctx.right_cache.add_folder(&folder).await?;
                    }
                    FolderSyncHunk::Create(folder, SyncDestination::Right) => {
                        ctx.right.add_folder(&folder).await?;
                    }
                    FolderSyncHunk::Uncache(folder, SyncDestination::Left) => {
                        ctx.left_cache.delete_folder(&folder).await?;
                    }
                    FolderSyncHunk::Delete(folder, SyncDestination::Left) => {
                        ctx.left.delete_folder(&folder).await?;
                    }
                    FolderSyncHunk::Uncache(folder, SyncDestination::Right) => {
                        ctx.right_cache.delete_folder(&folder).await?;
                    }
                    FolderSyncHunk::Delete(folder, SyncDestination::Right) => {
                        ctx.right.delete_folder(&folder).await?;
                    }
                };

                Ok(())
            };

            let output = task.await;

            SyncEvent::ProcessedFolderHunk(hunk.clone())
                .emit(&handler)
                .await;

            match output {
                Ok(()) => (hunk, None),
                Err(err) => (hunk, Some(err)),
            }
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

    SyncEvent::ProcessedAllFolderHunks
        .emit(&ctx_ref.handler)
        .await;

    Ok(report)
}

pub(crate) async fn expunge<L, R>(
    ctx_ref: Arc<SyncPoolContext<L::Context, R::Context>>,
    folders: &HashSet<String>,
) where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    FuturesUnordered::from_iter(folders.iter().map(|folder_ref| {
        let ctx = ctx_ref.clone();
        let folder = folder_ref.clone();
        let left_cached_expunge = async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.left_cache.expunge_folder(&folder).await
            }
        };

        let ctx = ctx_ref.clone();
        let folder = folder_ref.clone();
        let left_expunge = async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.left.expunge_folder(&folder).await
            }
        };

        let ctx = ctx_ref.clone();
        let folder = folder_ref.clone();
        let right_cached_expunge = async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.right_cache.expunge_folder(&folder).await
            }
        };

        let ctx = ctx_ref.clone();
        let folder = folder_ref.clone();
        let right_expunge = async move {
            if ctx.dry_run {
                Ok(())
            } else {
                ctx.right.expunge_folder(&folder).await
            }
        };

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

    SyncEvent::ExpungedAllFolders.emit(&ctx_ref.handler).await
}
