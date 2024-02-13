//! Module dedicated to emails synchronization runner.
//!
//! The core structure of this module is the [`EmailSyncRunner`].

use futures::lock::Mutex;
use log::{debug, trace};
use std::sync::Arc;

use crate::{
    account::sync::{AccountSyncProgress, AccountSyncProgressEvent},
    backend::{Backend, BackendBuilder, BackendContextBuilder},
    envelope::Id,
    flag::Flag,
    Result,
};

use super::*;

/// The email synchronization runner.
///
/// Acts a bit like a worker: the `run()` function takes a hunk from
/// the given patch and process it, then loops until there is no more
/// hunks available in the patch. The patch is in a mutex, which makes
/// the runner thread safe. Multiple runners can run in parallel.
pub struct EmailSyncRunner<B: BackendContextBuilder, LocalBackendBuilder: BackendContextBuilder> {
    /// The runner identifier, for logging purpose.
    pub id: usize,

    /// The local Maildir backend builder.
    pub local_builder: BackendBuilder<LocalBackendBuilder>,

    /// The remote backend builder.
    pub remote_builder: BackendBuilder<B>,

    /// The synchronization progress callback.
    pub on_progress: AccountSyncProgress,

    /// The patch this runner takes hunks from.
    pub patch: Arc<Mutex<Vec<Vec<EmailSyncHunk>>>>,
}

impl<RemoteBackendBuilder: BackendContextBuilder, LocalBackendBuilder: BackendContextBuilder>
    EmailSyncRunner<RemoteBackendBuilder, LocalBackendBuilder>
{
    async fn process_hunk(
        local: &Backend<LocalBackendBuilder::Context>,
        remote: &Backend<RemoteBackendBuilder::Context>,
        hunk: &EmailSyncHunk,
    ) -> Result<EmailSyncCachePatch>
    where
        RemoteBackendBuilder::Context: Send,
        LocalBackendBuilder::Context: Send,
    {
        let cache_hunks = match hunk {
            EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Left) => {
                let envelope = local.get_envelope(folder, &Id::single(id)).await?;
                vec![EmailSyncCacheHunk::Insert(
                    folder.clone(),
                    envelope.clone(),
                    SyncDestination::Left,
                )]
            }
            EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Right) => {
                let envelope = remote.get_envelope(folder, &Id::single(id)).await?;
                vec![EmailSyncCacheHunk::Insert(
                    folder.clone(),
                    envelope.clone(),
                    SyncDestination::Right,
                )]
            }
            EmailSyncHunk::CopyThenCache(
                folder,
                envelope,
                source,
                target,
                refresh_source_cache,
            ) => {
                let mut cache_hunks = vec![];
                let id = Id::single(&envelope.id);
                let emails = match source {
                    SyncDestination::Left => {
                        if *refresh_source_cache {
                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                folder.clone(),
                                envelope.clone(),
                                SyncDestination::Left,
                            ))
                        };
                        local.peek_messages(folder, &id).await?
                    }
                    SyncDestination::Right => {
                        if *refresh_source_cache {
                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                folder.clone(),
                                envelope.clone(),
                                SyncDestination::Right,
                            ))
                        };
                        remote.peek_messages(folder, &id).await?
                    }
                };

                let emails = emails.to_vec();
                let email = emails
                    .first()
                    .ok_or_else(|| Error::FindMessageError(envelope.id.clone()))?;

                match target {
                    SyncDestination::Left => {
                        let id = local
                            .add_message_with_flags(folder, email.raw()?, &envelope.flags)
                            .await?;
                        let envelope = local.get_envelope(folder, &Id::single(id)).await?;
                        cache_hunks.push(EmailSyncCacheHunk::Insert(
                            folder.clone(),
                            envelope.clone(),
                            SyncDestination::Left,
                        ));
                    }
                    SyncDestination::Right => {
                        let id = remote
                            .add_message_with_flags(folder, email.raw()?, &envelope.flags)
                            .await?;
                        let envelope = remote.get_envelope(folder, &Id::single(id)).await?;
                        cache_hunks.push(EmailSyncCacheHunk::Insert(
                            folder.clone(),
                            envelope.clone(),
                            SyncDestination::Right,
                        ));
                    }
                };
                cache_hunks
            }
            EmailSyncHunk::Uncache(folder, internal_id, SyncDestination::Left) => {
                vec![EmailSyncCacheHunk::Delete(
                    folder.clone(),
                    internal_id.clone(),
                    SyncDestination::Left,
                )]
            }
            EmailSyncHunk::Delete(folder, id, SyncDestination::Left) => {
                local
                    .add_flag(folder, &Id::single(id), Flag::Deleted)
                    .await?;
                vec![]
            }
            EmailSyncHunk::Uncache(folder, internal_id, SyncDestination::Right) => {
                vec![EmailSyncCacheHunk::Delete(
                    folder.clone(),
                    internal_id.clone(),
                    SyncDestination::Right,
                )]
            }
            EmailSyncHunk::Delete(folder, id, SyncDestination::Right) => {
                remote
                    .add_flag(folder, &Id::single(id), Flag::Deleted)
                    .await?;
                vec![]
            }
            EmailSyncHunk::UpdateCachedFlags(folder, envelope, SyncDestination::Left) => {
                vec![
                    EmailSyncCacheHunk::Delete(
                        folder.clone(),
                        envelope.id.clone(),
                        SyncDestination::Left,
                    ),
                    EmailSyncCacheHunk::Insert(
                        folder.clone(),
                        envelope.clone(),
                        SyncDestination::Left,
                    ),
                ]
            }
            EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Left) => {
                local
                    .set_flags(folder, &Id::single(&envelope.id), &envelope.flags)
                    .await?;
                vec![]
            }
            EmailSyncHunk::UpdateCachedFlags(folder, envelope, SyncDestination::Right) => {
                vec![
                    EmailSyncCacheHunk::Delete(
                        folder.clone(),
                        envelope.id.clone(),
                        SyncDestination::Right,
                    ),
                    EmailSyncCacheHunk::Insert(
                        folder.clone(),
                        envelope.clone(),
                        SyncDestination::Right,
                    ),
                ]
            }
            EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Right) => {
                remote
                    .set_flags(folder, &Id::single(&envelope.id), &envelope.flags)
                    .await?;
                vec![]
            }
        };

        Ok(cache_hunks)
    }

    /// Runs the synchronization worker and stops when there is no
    /// more hunks in the patch.
    pub async fn run(&self) -> Result<EmailSyncReport> {
        let mut report = EmailSyncReport::default();
        let local = self.local_builder.clone().build().await?;
        let remote = self.remote_builder.clone().build().await?;

        loop {
            // wrap in a block to free the lock as quickly as possible
            let hunks = {
                let mut lock = self.patch.lock().await;
                lock.pop()
            };

            match hunks {
                None => break,
                Some(hunks) => {
                    for hunk in hunks {
                        trace!("sync runner {} processing envelope hunk: {hunk:?}", self.id);

                        match Self::process_hunk(&local, &remote, &hunk).await {
                            Ok(cache_hunks) => {
                                report.patch.push((hunk.clone(), None));
                                report.cache_patch.0.extend(cache_hunks);
                            }
                            Err(err) => {
                                debug!("error while processing hunk {hunk:?}: {err}");
                                debug!("{err:?}");
                                report.patch.push((hunk.clone(), Some(err)));
                            }
                        };

                        self.on_progress
                            .emit(AccountSyncProgressEvent::ApplyEnvelopeHunk(hunk));
                    }
                }
            }
        }

        Ok(report)
    }
}
