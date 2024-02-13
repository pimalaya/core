//! # Email synchronization
//!
//! Module dedicated to email synchronization.

pub mod hunk;
pub mod patch;
pub mod report;

use futures::{stream::FuturesUnordered, StreamExt};
use log::debug;
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::{
    backend::BackendContextBuilder,
    envelope::{Envelope, Id},
    flag::Flag,
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
    pool: &ThreadPool<SyncPoolContext<L::Context, R::Context>>,
    folders: &HashSet<String>,
) -> Result<EmailSyncReport>
where
    L: BackendContextBuilder + 'static,
    R: BackendContextBuilder + 'static,
{
    let mut report = EmailSyncReport::default();

    let patch = FuturesUnordered::from_iter(folders.iter().map(|folder_ref| {
        let folder = folder_ref.clone();
        let left_cached_envelopes = pool.exec(|ctx| async move {
            let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                ctx.left_cache
                    .list_envelopes(&folder, 0, 0)
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

            SyncEvent::ListedLeftCachedEnvelopes(folder.clone(), envelopes.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(envelopes)
        });

        let folder = folder_ref.clone();
        let left_envelopes = pool.exec(|ctx| async move {
            let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                ctx.left
                    .list_envelopes(&folder, 0, 0)
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

            SyncEvent::ListedLeftEnvelopes(folder.clone(), envelopes.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(envelopes)
        });

        let folder = folder_ref.clone();
        let right_cached_envelopes = pool.exec(|ctx| async move {
            let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                ctx.right_cache
                    .list_envelopes(&folder, 0, 0)
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

            SyncEvent::ListedRightCachedEnvelopes(folder.clone(), envelopes.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(envelopes)
        });

        let folder = folder_ref.clone();
        let right_envelopes = pool.exec(|ctx| async move {
            let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                ctx.right
                    .list_envelopes(&folder, 0, 0)
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

            SyncEvent::ListedRightEnvelopes(folder.clone(), envelopes.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(envelopes)
        });

        async move {
            let (left_cached_envelopes, left_envelopes, right_cached_envelopes, right_envelopes) =
                tokio::try_join!(
                    left_cached_envelopes,
                    left_envelopes,
                    right_cached_envelopes,
                    right_envelopes
                )?;

            let patch = patch::build(
                folder_ref,
                left_cached_envelopes,
                left_envelopes,
                right_cached_envelopes,
                right_envelopes,
            );

            Result::Ok(patch)
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
    .fold(Vec::new(), |mut patch, p| async {
        patch.extend(p.into_iter().flatten());
        patch
    })
    .await;

    pool.exec(|ctx| async move {
        SyncEvent::ListedAllEnvelopes.emit(&ctx.handler).await;
    })
    .await;

    report.patch = FuturesUnordered::from_iter(patch.into_iter().map(|hunk| {
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
                        let msg = Vec::<u8>::try_from(envelope)?;
                        ctx.left_cache
                            .add_message_with_flags(&folder, &msg, &flags)
                            .await?;
                        Ok(())
                    }
                    EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Right) => {
                        let envelope = ctx.right.get_envelope(&folder, &Id::single(id)).await?;
                        let flags = envelope.flags.clone();
                        let msg = Vec::<u8>::try_from(envelope)?;
                        ctx.right_cache
                            .add_message_with_flags(&folder, &msg, &flags)
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
                                    let msg = Vec::<u8>::try_from(envelope.clone())?;
                                    ctx.left_cache
                                        .add_message_with_flags(&folder, &msg, &flags)
                                        .await?;
                                };
                                ctx.left.peek_messages(&folder, &id).await?
                            }
                            SyncDestination::Right => {
                                if refresh_source_cache {
                                    let flags = envelope.flags.clone();
                                    let msg = Vec::<u8>::try_from(envelope.clone())?;
                                    ctx.right_cache
                                        .add_message_with_flags(&folder, &msg, &flags)
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
                                    .add_message_with_flags(&folder, msg.raw()?, &envelope.flags)
                                    .await?;
                                let envelope =
                                    ctx.left.get_envelope(&folder, &Id::single(id)).await?;
                                let flags = envelope.flags.clone();
                                let msg = Vec::<u8>::try_from(envelope)?;
                                ctx.left_cache
                                    .add_message_with_flags(&folder, &msg, &flags)
                                    .await?;
                                Ok(())
                            }
                            SyncDestination::Right => {
                                let id = ctx
                                    .right
                                    .add_message_with_flags(&folder, msg.raw()?, &envelope.flags)
                                    .await?;
                                let envelope =
                                    ctx.right.get_envelope(&folder, &Id::single(id)).await?;
                                let flags = envelope.flags.clone();
                                let msg = Vec::<u8>::try_from(envelope)?;
                                ctx.right_cache
                                    .add_message_with_flags(&folder, &msg, &flags)
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
                    EmailSyncHunk::UpdateCachedFlags(folder, envelope, SyncDestination::Left) => {
                        ctx.left_cache
                            .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                            .await
                    }
                    EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Left) => {
                        ctx.left
                            .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                            .await
                    }
                    EmailSyncHunk::UpdateCachedFlags(folder, envelope, SyncDestination::Right) => {
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
    }))
    .collect::<Vec<_>>()
    .await;

    Ok(report)
}
