//! Module dedicated to emails synchronization runner.
//!
//! The core structure of this module is the [`EmailSyncRunner`].

use futures::lock::Mutex;
use log::{trace, warn};
use std::sync::Arc;

use crate::{
    account::sync::{AccountSyncProgress, AccountSyncProgressEvent, LocalBackendBuilder},
    backend::{BackendBuilderV2, BackendContextBuilder, BackendV2},
    email::{envelope::Id, Flag},
    maildir::MaildirSessionSync,
    Result,
};

use super::*;

/// The email synchronization runner.
///
/// Acts a bit like a worker: the `run()` function takes a hunk from
/// the given patch and process it, then loops until there is no more
/// hunks available in the patch. The patch is in a mutex, which makes
/// the runner thread safe. Multiple runners can run in parallel.
pub struct EmailSyncRunner<B: BackendContextBuilder> {
    /// The runner identifier, for logging purpose.
    pub id: usize,

    /// The local Maildir backend builder.
    pub local_builder: LocalBackendBuilder,

    /// The remote backend builder.
    pub remote_builder: BackendBuilderV2<B>,

    /// The synchronization progress callback.
    pub on_progress: AccountSyncProgress,

    /// The patch this runner takes hunks from.
    pub patch: Arc<Mutex<Vec<Vec<EmailSyncHunk>>>>,
}

impl<B: BackendContextBuilder> EmailSyncRunner<B> {
    async fn process_hunk(
        local: &BackendV2<MaildirSessionSync>,
        remote: &BackendV2<B::Context>,
        hunk: &EmailSyncHunk,
    ) -> Result<EmailSyncCachePatch> {
        let cache_hunks = match hunk {
            EmailSyncHunk::GetThenCache(folder, internal_id, Destination::Local) => {
                let envelope = local.get_envelope(&folder, &internal_id).await?;
                vec![EmailSyncCacheHunk::Insert(
                    folder.clone(),
                    envelope.clone(),
                    Destination::Local,
                )]
            }
            EmailSyncHunk::GetThenCache(folder, internal_id, Destination::Remote) => {
                let envelope = remote.get_envelope(&folder, &internal_id).await?;
                vec![EmailSyncCacheHunk::Insert(
                    folder.clone(),
                    envelope.clone(),
                    Destination::Remote,
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
                    Destination::Local => {
                        if *refresh_source_cache {
                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                folder.clone(),
                                envelope.clone(),
                                Destination::Local,
                            ))
                        };
                        local.peek_messages(&folder, &id).await?
                    }
                    Destination::Remote => {
                        if *refresh_source_cache {
                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                folder.clone(),
                                envelope.clone(),
                                Destination::Remote,
                            ))
                        };
                        remote.peek_messages(&folder, &id).await?
                    }
                };

                let emails = emails.to_vec();
                let email = emails
                    .first()
                    .ok_or_else(|| Error::FindEmailError(envelope.id.clone()))?;

                match target {
                    Destination::Local => {
                        let internal_id = local
                            .add_raw_message_with_flags(&folder, email.raw()?, &envelope.flags)
                            .await?;
                        let envelope = local.get_envelope(&folder, &internal_id).await?;
                        cache_hunks.push(EmailSyncCacheHunk::Insert(
                            folder.clone(),
                            envelope.clone(),
                            Destination::Local,
                        ));
                    }
                    Destination::Remote => {
                        let internal_id = remote
                            .add_raw_message_with_flags(&folder, email.raw()?, &envelope.flags)
                            .await?;
                        let envelope = remote.get_envelope(&folder, &internal_id).await?;
                        cache_hunks.push(EmailSyncCacheHunk::Insert(
                            folder.clone(),
                            envelope.clone(),
                            Destination::Remote,
                        ));
                    }
                };
                cache_hunks
            }
            EmailSyncHunk::Uncache(folder, internal_id, Destination::Local) => {
                vec![EmailSyncCacheHunk::Delete(
                    folder.clone(),
                    internal_id.clone(),
                    Destination::Local,
                )]
            }
            EmailSyncHunk::Delete(folder, id, Destination::Local) => {
                local
                    .add_flag(&folder, &Id::single(id), Flag::Deleted)
                    .await?;
                vec![]
            }
            EmailSyncHunk::Uncache(folder, internal_id, Destination::Remote) => {
                vec![EmailSyncCacheHunk::Delete(
                    folder.clone(),
                    internal_id.clone(),
                    Destination::Remote,
                )]
            }
            EmailSyncHunk::Delete(folder, id, Destination::Remote) => {
                remote
                    .add_flag(&folder, &Id::single(id), Flag::Deleted)
                    .await?;
                vec![]
            }
            EmailSyncHunk::UpdateCachedFlags(folder, envelope, Destination::Local) => {
                vec![
                    EmailSyncCacheHunk::Delete(
                        folder.clone(),
                        envelope.id.clone(),
                        Destination::Local,
                    ),
                    EmailSyncCacheHunk::Insert(
                        folder.clone(),
                        envelope.clone(),
                        Destination::Local,
                    ),
                ]
            }
            EmailSyncHunk::UpdateFlags(folder, envelope, Destination::Local) => {
                local
                    .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                    .await?;
                vec![]
            }
            EmailSyncHunk::UpdateCachedFlags(folder, envelope, Destination::Remote) => {
                vec![
                    EmailSyncCacheHunk::Delete(
                        folder.clone(),
                        envelope.id.clone(),
                        Destination::Remote,
                    ),
                    EmailSyncCacheHunk::Insert(
                        folder.clone(),
                        envelope.clone(),
                        Destination::Remote,
                    ),
                ]
            }
            EmailSyncHunk::UpdateFlags(folder, envelope, Destination::Remote) => {
                remote
                    .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
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
                                warn!("error while processing hunk {hunk:?}, skipping it: {err:?}");
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
