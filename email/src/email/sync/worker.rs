//! Module dedicated to emails synchronization runner.
//!
//! The core structure of this module is the [`EmailSyncRunner`].

use futures::{lock::Mutex, stream, StreamExt};
use log::debug;
use std::sync::Arc;
use thiserror::Error;

use crate::{
    backend::{BackendBuilder, BackendContextBuilder},
    email::sync::EmailSyncEvent,
    envelope::Id,
    flag::Flag,
    maildir::MaildirContextBuilder,
    sync::SyncDestination,
    Result,
};

use super::{EmailSyncEventHandler, EmailSyncHunk, EmailSyncReport};

/// Errors related to email synchronization.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find message associated to envelope {0}")]
    FindMessageError(String),
}

pub async fn process_patch<L, R>(
    left_builder: BackendBuilder<L>,
    left_cached_builder: BackendBuilder<MaildirContextBuilder>,
    right_builder: BackendBuilder<R>,
    right_cached_builder: BackendBuilder<MaildirContextBuilder>,
    handler: Option<Arc<EmailSyncEventHandler>>,
    patch: Vec<Vec<EmailSyncHunk>>,
    pool_size: usize,
) -> EmailSyncReport
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let patch = Arc::new(Mutex::new(patch));

    stream::iter(0..pool_size)
        .map(|id| EmailSyncWorker {
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
                        debug!("error during email sync from worker {id}: {err:?}");
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
        .fold(EmailSyncReport::default(), |mut r1, r2| async {
            r1.patch.extend(r2.patch);
            r1
        })
        .await
}

/// The email synchronization worker.
///
/// Acts a bit like a worker: the `run()` function takes a hunk from
/// the given patch and process it, then loops until there is no more
/// hunks available in the patch. The patch is in a mutex, which makes
/// the runner thread safe. Multiple runners can run in parallel.
pub struct EmailSyncWorker<L: BackendContextBuilder, R: BackendContextBuilder> {
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
    pub handler: Option<Arc<EmailSyncEventHandler>>,

    /// The patch this runner takes hunks from.
    pub patch: Arc<Mutex<Vec<Vec<EmailSyncHunk>>>>,
}

impl<L: BackendContextBuilder, R: BackendContextBuilder> EmailSyncWorker<L, R> {
    /// Runs the synchronization worker and stops when there is no
    /// more hunks in the patch.
    pub async fn process_hunks(self) -> Result<EmailSyncReport> {
        let mut report = EmailSyncReport::default();

        let id = self.id;
        let left = self.left_builder.build().await?;
        let _left_cached = self.left_cached_builder.build().await?;
        let right = self.right_builder.build().await?;
        let _right_cached = self.right_cached_builder.build().await?;

        loop {
            // wrap in a block to free the lock as quickly as possible
            let hunks = {
                let mut lock = self.patch.lock().await;
                lock.pop()
            };

            match hunks {
                None => {
                    debug!("email sync worker {id} stopping work");
                    break;
                }
                Some(hunks) => {
                    for hunk in hunks {
                        debug!("email sync worker {id} processing {hunk:?}");

                        let res = async {
                            match &hunk {
                                EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Left) => {
                                    let _envelope =
                                        left.get_envelope(folder, &Id::single(id)).await?;
                                    // TODO: insert left cache
                                }
                                EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Right) => {
                                    let _envelope =
                                        right.get_envelope(folder, &Id::single(id)).await?;
                                    // TODO: insert right cache
                                }
                                EmailSyncHunk::CopyThenCache(
                                    folder,
                                    envelope,
                                    source,
                                    target,
                                    refresh_source_cache,
                                ) => {
                                    let id = Id::single(&envelope.id);
                                    let msgs = match source {
                                        SyncDestination::Left => {
                                            if *refresh_source_cache {
                                                // TODO: insert left cache
                                            };
                                            left.peek_messages(folder, &id).await?
                                        }
                                        SyncDestination::Right => {
                                            if *refresh_source_cache {
                                                // TODO: insert right cache
                                            };
                                            right.peek_messages(folder, &id).await?
                                        }
                                    };

                                    let msgs = msgs.to_vec();
                                    let msg = msgs.first().ok_or_else(|| {
                                        Error::FindMessageError(envelope.id.clone())
                                    })?;

                                    match target {
                                        SyncDestination::Left => {
                                            let id = left
                                                .add_message_with_flags(
                                                    folder,
                                                    msg.raw()?,
                                                    &envelope.flags,
                                                )
                                                .await?;
                                            let _envelope =
                                                left.get_envelope(folder, &Id::single(id)).await?;
                                            // TODO: insert left cache
                                        }
                                        SyncDestination::Right => {
                                            let id = right
                                                .add_message_with_flags(
                                                    folder,
                                                    msg.raw()?,
                                                    &envelope.flags,
                                                )
                                                .await?;
                                            let _envelope =
                                                right.get_envelope(folder, &Id::single(id)).await?;
                                            // TODO: insert right cache
                                        }
                                    };
                                }
                                EmailSyncHunk::Uncache(
                                    _folder,
                                    _internal_id,
                                    SyncDestination::Left,
                                ) => {
                                    // TODO: remove left cache
                                }
                                EmailSyncHunk::Delete(folder, id, SyncDestination::Left) => {
                                    left.add_flag(folder, &Id::single(id), Flag::Deleted)
                                        .await?;
                                }
                                EmailSyncHunk::Uncache(
                                    _folder,
                                    _internal_id,
                                    SyncDestination::Right,
                                ) => {
                                    // TODO: remove right cache
                                }
                                EmailSyncHunk::Delete(folder, id, SyncDestination::Right) => {
                                    right
                                        .add_flag(folder, &Id::single(id), Flag::Deleted)
                                        .await?;
                                }
                                EmailSyncHunk::UpdateCachedFlags(
                                    _folder,
                                    _envelope,
                                    SyncDestination::Left,
                                ) => {
                                    // TODO: replace left cache
                                }
                                EmailSyncHunk::UpdateFlags(
                                    folder,
                                    envelope,
                                    SyncDestination::Left,
                                ) => {
                                    left.set_flags(
                                        folder,
                                        &Id::single(&envelope.id),
                                        &envelope.flags,
                                    )
                                    .await?;
                                }
                                EmailSyncHunk::UpdateCachedFlags(
                                    _folder,
                                    _envelope,
                                    SyncDestination::Right,
                                ) => {
                                    // TODO: replace right cache
                                }
                                EmailSyncHunk::UpdateFlags(
                                    folder,
                                    envelope,
                                    SyncDestination::Right,
                                ) => {
                                    right
                                        .set_flags(
                                            folder,
                                            &Id::single(&envelope.id),
                                            &envelope.flags,
                                        )
                                        .await?;
                                }
                            }

                            Ok(())
                        };

                        match res.await {
                            Ok(()) => {
                                report.patch.push((hunk.clone(), None));
                            }
                            Err(err) => {
                                report.patch.push((hunk.clone(), Some(err)));
                            }
                        };

                        EmailSyncEvent::ProcessedEmailHunk(hunk)
                            .emit(&self.handler)
                            .await;
                    }
                }
            }
        }

        Ok(report)
    }
}
