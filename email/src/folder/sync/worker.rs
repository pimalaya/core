//! Module dedicated to folders synchronization runner.
//!
//! The core structure of this module is the [`FolderSyncRunner`].

use futures::{lock::Mutex, stream, StreamExt};
use log::debug;
use std::sync::Arc;

use crate::{
    backend::{BackendBuilder, BackendContextBuilder},
    folder::sync::FolderSyncEvent,
    maildir::MaildirContextBuilder,
    sync::SyncDestination,
    Result,
};

use super::{FolderSyncEventHandler, FolderSyncHunk, FolderSyncPatch, FolderSyncReport};

pub async fn process_patch<L, R>(
    left_builder: BackendBuilder<L>,
    left_cached_builder: BackendBuilder<MaildirContextBuilder>,
    right_builder: BackendBuilder<R>,
    right_cached_builder: BackendBuilder<MaildirContextBuilder>,
    handler: Option<Arc<FolderSyncEventHandler>>,
    patch: FolderSyncPatch,
    pool_size: usize,
) -> FolderSyncReport
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let patch = Arc::new(Mutex::new(patch));

    stream::iter(0..pool_size)
        .map(|id| FolderSyncWorker {
            id,
            left_builder: left_builder.clone(),
            left_cached_builder: left_cached_builder.clone(),
            right_builder: right_builder.clone(),
            right_cached_builder: right_cached_builder.clone(),
            handler: handler.clone(),
            patch: patch.clone(),
        })
        .map(|worker| {
            tokio::spawn(async move {
                let id = worker.id;
                match worker.process_hunks().await {
                    Ok(report) => Some(report),
                    Err(err) => {
                        debug!("error during folder sync from worker {id}: {err:?}");
                        None
                    }
                }
            })
        })
        .buffer_unordered(pool_size)
        .filter_map(|report| async {
            if let Ok(Some(report)) = report {
                Some(report)
            } else {
                None
            }
        })
        .fold(FolderSyncReport::default(), |mut r1, r2| async {
            r1.patch.extend(r2.patch);
            r1
        })
        .await
}

/// The folder synchronization worker.
///
/// Acts a bit like a worker: the `run()` function takes a hunk from
/// the given patch and process it, then loops until there is no more
/// hunks available in the patch. The patch is in a mutex, which makes
/// the runner thread safe. Multiple runners can run in parallel.
pub struct FolderSyncWorker<L: BackendContextBuilder, R: BackendContextBuilder> {
    /// The runner identifier, for logging purpose.
    pub id: usize,

    /// The left backend builder.
    pub left_builder: BackendBuilder<L>,

    /// The left cached backend builder.
    pub left_cached_builder: BackendBuilder<MaildirContextBuilder>,

    /// The right backend builder.
    pub right_builder: BackendBuilder<R>,

    /// The right cached backend builder.
    pub right_cached_builder: BackendBuilder<MaildirContextBuilder>,

    /// The synchronization progress callback.
    pub handler: Option<Arc<FolderSyncEventHandler>>,

    /// The patch this runner takes hunks from.
    pub patch: Arc<Mutex<Vec<FolderSyncHunk>>>,
}

impl<L: BackendContextBuilder, R: BackendContextBuilder> FolderSyncWorker<L, R> {
    /// Runs the synchronization worker and stops when there is no
    /// more hunks in the patch.
    pub async fn process_hunks(self) -> Result<FolderSyncReport> {
        let mut report = FolderSyncReport::default();

        let id = self.id;
        let left = self.left_builder.build().await?;
        let left_cached = self.left_cached_builder.build().await?;
        let right = self.right_builder.build().await?;
        let right_cached = self.right_cached_builder.build().await?;

        loop {
            // wrap in a block to free the lock as quickly as possible
            let hunk = {
                let mut lock = self.patch.lock().await;
                lock.pop()
            };

            match hunk {
                None => {
                    debug!("folder sync worker {id} stopping work");
                    break;
                }
                Some(hunk) => {
                    debug!("folder sync worker {id} processing {hunk:?}");

                    let res = match &hunk {
                        FolderSyncHunk::Cache(folder, SyncDestination::Left) => {
                            left_cached.add_folder(folder).await
                        }
                        FolderSyncHunk::Create(folder, SyncDestination::Left) => {
                            left.add_folder(folder).await
                        }
                        FolderSyncHunk::Cache(folder, SyncDestination::Right) => {
                            right_cached.add_folder(folder).await
                        }
                        FolderSyncHunk::Create(folder, SyncDestination::Right) => {
                            right.add_folder(folder).await
                        }
                        FolderSyncHunk::Uncache(folder, SyncDestination::Left) => {
                            left_cached.delete_folder(folder).await
                        }
                        FolderSyncHunk::Delete(folder, SyncDestination::Left) => {
                            left.delete_folder(folder).await
                        }
                        FolderSyncHunk::Uncache(folder, SyncDestination::Right) => {
                            right_cached.delete_folder(folder).await
                        }
                        FolderSyncHunk::Delete(folder, SyncDestination::Right) => {
                            right.delete_folder(folder).await
                        }
                    };

                    match res {
                        Ok(()) => {
                            report.patch.push((hunk.clone(), None));
                        }
                        Err(err) => {
                            report.patch.push((hunk.clone(), Some(err)));
                        }
                    };

                    FolderSyncEvent::ProcessedFolderHunk(hunk)
                        .emit(&self.handler)
                        .await;
                }
            }
        }

        Ok(report)
    }
}
