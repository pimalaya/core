//! # Email synchronization
//!
//! Module dedicated to email synchronization.

pub mod hunk;
pub mod patch;
pub mod report;

use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, trace};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
};
use thiserror::Error;

use crate::{
    backend::context::BackendContextBuilder,
    envelope::{get::GetEnvelope, list::ListEnvelopes, Envelope, Id},
    flag::{add::AddFlags, set::SetFlags, Flag},
    message::{add::AddMessage, peek::PeekMessages},
    sync::{pool::SyncPoolContext, SyncDestination, SyncEvent},
    thread_pool::ThreadPool,
    Result,
};

use self::{hunk::EmailSyncHunk, report::EmailSyncReport};

/// Errors related to email synchronization.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find message associated to envelope {0}")]
    FindMessageError(String),
}

pub(crate) async fn sync<L, R>(
    pool: Arc<ThreadPool<SyncPoolContext<L::Context, R::Context>>>,
    folders: &HashSet<String>,
) -> Result<EmailSyncReport>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let mut report = EmailSyncReport::default();

    let patch = FuturesUnordered::from_iter(folders.iter().map(|folder| {
        let pool_ref = pool.clone();
        let folder_ref = folder.clone();
        let left_cached_envelopes = tokio::spawn(async move {
            pool_ref
                .exec(|ctx| async move {
                    let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                        ctx.left_cache
                            .list_envelopes(&folder_ref, 0, 0)
                            .await
                            .or_else(|err| {
                                if ctx.dry_run {
                                    Ok(Default::default())
                                } else {
                                    Err(err)
                                }
                            })?
                            .into_iter()
                            .map(|e| (e.message_id.clone(), e)),
                    );

                    SyncEvent::ListedLeftCachedEnvelopes(folder_ref.clone(), envelopes.len())
                        .emit(&ctx.handler)
                        .await;

                    Result::Ok(envelopes)
                })
                .await
        });

        let pool_ref = pool.clone();
        let folder_ref = folder.clone();
        let left_envelopes = tokio::spawn(async move {
            pool_ref
                .exec(|ctx| async move {
                    let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                        ctx.left
                            .list_envelopes(&folder_ref, 0, 0)
                            .await
                            .or_else(|err| {
                                if ctx.dry_run {
                                    Ok(Default::default())
                                } else {
                                    Err(err)
                                }
                            })?
                            .into_iter()
                            .map(|e| (e.message_id.clone(), e)),
                    );

                    SyncEvent::ListedLeftEnvelopes(folder_ref.clone(), envelopes.len())
                        .emit(&ctx.handler)
                        .await;

                    Result::Ok(envelopes)
                })
                .await
        });

        let pool_ref = pool.clone();
        let folder_ref = folder.clone();
        let right_cached_envelopes = tokio::spawn(async move {
            pool_ref
                .exec(|ctx| async move {
                    let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                        ctx.right_cache
                            .list_envelopes(&folder_ref, 0, 0)
                            .await
                            .or_else(|err| {
                                if ctx.dry_run {
                                    Ok(Default::default())
                                } else {
                                    Err(err)
                                }
                            })?
                            .into_iter()
                            .map(|e| (e.message_id.clone(), e)),
                    );

                    SyncEvent::ListedRightCachedEnvelopes(folder_ref.clone(), envelopes.len())
                        .emit(&ctx.handler)
                        .await;

                    Result::Ok(envelopes)
                })
                .await
        });

        let pool_ref = pool.clone();
        let folder_ref = folder.clone();
        let right_envelopes = tokio::spawn(async move {
            pool_ref
                .exec(|ctx| async move {
                    let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                        ctx.right
                            .list_envelopes(&folder_ref, 0, 0)
                            .await
                            .or_else(|err| {
                                if ctx.dry_run {
                                    Ok(Default::default())
                                } else {
                                    Err(err)
                                }
                            })?
                            .into_iter()
                            .map(|e| (e.message_id.clone(), e)),
                    );

                    SyncEvent::ListedRightEnvelopes(folder_ref.clone(), envelopes.len())
                        .emit(&ctx.handler)
                        .await;

                    Result::Ok(envelopes)
                })
                .await
        });

        async move {
            let envelopes = tokio::try_join!(
                left_cached_envelopes,
                left_envelopes,
                right_cached_envelopes,
                right_envelopes
            );

            Result::Ok((folder.clone(), envelopes))
        }
    }))
    .filter_map(|patch| async {
        let task = async {
            let (folder, envelopes) = patch?;
            let (lc, l, rc, r) = envelopes?;
            let patch = patch::build(&folder, lc?, l?, rc?, r?);
            Result::Ok((folder, patch))
        };
        match task.await {
            Ok(patch) => Some(patch),
            Err(err) => {
                debug!("cannot generate email patch: {err}");
                trace!("{err:?}");
                None
            }
        }
    })
    .fold(BTreeMap::new(), |mut patch, (folder, p)| async {
        patch.insert(folder, p.into_iter().flatten().collect::<BTreeSet<_>>());
        patch
    })
    .await;

    let patch_clone = patch.clone();
    pool.exec(|ctx| async move {
        SyncEvent::GeneratedEmailPatch(patch_clone)
            .emit(&ctx.handler)
            .await;
    })
    .await;

    report.patch = FuturesUnordered::from_iter(patch.into_values().flatten().map(|hunk| {
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
                        EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Left) => {
                            let envelope = ctx.left.get_envelope(&folder, &Id::single(id)).await?;
                            let flags = envelope.flags.clone();
                            let msg = format!("Message-ID: {}\n\n", envelope.message_id);
                            ctx.left_cache
                                .add_message_with_flags(&folder, msg.as_bytes(), &flags)
                                .await?;
                            Ok(())
                        }
                        EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Right) => {
                            let envelope = ctx.right.get_envelope(&folder, &Id::single(id)).await?;
                            let flags = envelope.flags.clone();
                            let msg = format!("Message-ID: {}\n\n", envelope.message_id);
                            ctx.right_cache
                                .add_message_with_flags(&folder, msg.as_bytes(), &flags)
                                .await?;
                            Ok(())
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
                                    if refresh_source_cache {
                                        let flags = envelope.flags.clone();
                                        let msg =
                                            format!("Message-ID: {}\n\n", envelope.message_id);
                                        ctx.left_cache
                                            .add_message_with_flags(&folder, msg.as_bytes(), &flags)
                                            .await?;
                                    };
                                    ctx.left.peek_messages(&folder, &id).await?
                                }
                                SyncDestination::Right => {
                                    if refresh_source_cache {
                                        let flags = envelope.flags.clone();
                                        let msg =
                                            format!("Message-ID: {}\n\n", envelope.message_id);
                                        ctx.right_cache
                                            .add_message_with_flags(&folder, msg.as_bytes(), &flags)
                                            .await?;
                                    };
                                    ctx.right.peek_messages(&folder, &id).await?
                                }
                            };

                            let msgs = msgs.to_vec();
                            let msg = msgs
                                .first()
                                .ok_or_else(|| Error::FindMessageError(envelope.id.clone()))?;

                            match target {
                                SyncDestination::Left => {
                                    let id = ctx
                                        .left
                                        .add_message_with_flags(
                                            &folder,
                                            msg.raw()?,
                                            &envelope.flags,
                                        )
                                        .await?;
                                    let envelope =
                                        ctx.left.get_envelope(&folder, &Id::single(id)).await?;
                                    let flags = envelope.flags.clone();
                                    let msg = format!("Message-ID: {}\n\n", envelope.message_id);
                                    ctx.left_cache
                                        .add_message_with_flags(&folder, msg.as_bytes(), &flags)
                                        .await?;
                                    Ok(())
                                }
                                SyncDestination::Right => {
                                    let id = ctx
                                        .right
                                        .add_message_with_flags(
                                            &folder,
                                            msg.raw()?,
                                            &envelope.flags,
                                        )
                                        .await?;
                                    let envelope =
                                        ctx.right.get_envelope(&folder, &Id::single(id)).await?;
                                    let flags = envelope.flags.clone();
                                    let msg = format!("Message-ID: {}\n\n", envelope.message_id);
                                    ctx.right_cache
                                        .add_message_with_flags(&folder, msg.as_bytes(), &flags)
                                        .await?;
                                    Ok(())
                                }
                            }
                        }
                        EmailSyncHunk::Uncache(folder, id, SyncDestination::Left) => {
                            ctx.left_cache
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::Delete(folder, id, SyncDestination::Left) => {
                            ctx.left
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::Uncache(folder, id, SyncDestination::Right) => {
                            ctx.right_cache
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::Delete(folder, id, SyncDestination::Right) => {
                            ctx.right
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::UpdateCachedFlags(
                            folder,
                            envelope,
                            SyncDestination::Left,
                        ) => {
                            ctx.left_cache
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                        EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Left) => {
                            ctx.left
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                        EmailSyncHunk::UpdateCachedFlags(
                            folder,
                            envelope,
                            SyncDestination::Right,
                        ) => {
                            ctx.right_cache
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                        EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Right) => {
                            ctx.right
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                    }
                };

                async move {
                    let output = task.await;

                    SyncEvent::ProcessedEmailHunk(hunk.clone())
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
    .filter_map(|res| async {
        match res {
            Ok(res) => Some(res),
            Err(err) => {
                debug!("cannot process email hunk: {err}");
                trace!("{err:?}");
                None
            }
        }
    })
    .collect::<Vec<_>>()
    .await;

    pool.exec(|ctx| async move {
        SyncEvent::ProcessedAllEmailHunks.emit(&ctx.handler).await;
    })
    .await;

    Ok(report)
}
