//! Module dedicated to emails synchronization runner.
//!
//! The core structure of this module is the [`EmailSyncRunner`].

use futures::lock::Mutex;
use log::{trace, warn};
use std::sync::Arc;

use crate::{
    account::sync::{AccountSyncProgress, AccountSyncProgressEvent},
    backend::{Backend, BackendBuilder, MaildirBackend, MaildirBackendBuilder},
    Result,
};

use super::*;

/// The email synchronization runner.
///
/// Acts a bit like a worker: the `run()` function takes a hunk from
/// the given patch and process it, then loops until there is no more
/// hunks available in the patch. The patch is in a
/// [`std::sync::Mutex`], which makes the runner thread safe. Multiple
/// runner can run in parallel.
pub struct EmailSyncRunner {
    /// The runner identifier, for logging purpose.
    pub id: usize,

    /// The local Maildir backend builder.
    pub local_builder: Arc<MaildirBackendBuilder>,

    /// The remote backend builder.
    pub remote_builder: Arc<BackendBuilder>,

    /// The synchronization progress callback.
    pub on_progress: AccountSyncProgress,

    /// The patch this runner takes hunks from.
    pub patch: Arc<Mutex<Vec<Vec<EmailSyncHunk>>>>,
}

impl EmailSyncRunner {
    async fn process_hunk(
        local: &mut MaildirBackend,
        remote: &mut dyn Backend,
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
                let internal_ids = vec![envelope.id.as_str()];
                let emails = match source {
                    Destination::Local => {
                        if *refresh_source_cache {
                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                folder.clone(),
                                envelope.clone(),
                                Destination::Local,
                            ))
                        };
                        local.preview_emails(&folder, internal_ids).await?
                    }
                    Destination::Remote => {
                        if *refresh_source_cache {
                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                folder.clone(),
                                envelope.clone(),
                                Destination::Remote,
                            ))
                        };
                        remote.preview_emails(&folder, internal_ids).await?
                    }
                };

                let emails = emails.to_vec();
                let email = emails
                    .first()
                    .ok_or_else(|| Error::FindEmailError(envelope.id.clone()))?;

                match target {
                    Destination::Local => {
                        let internal_id = local
                            .add_email(&folder, email.raw()?, &envelope.flags)
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
                            .add_email(&folder, email.raw()?, &envelope.flags)
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
            EmailSyncHunk::Delete(folder, internal_id, Destination::Local) => {
                local
                    .mark_emails_as_deleted(&folder, vec![&internal_id])
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
            EmailSyncHunk::Delete(folder, internal_id, Destination::Remote) => {
                remote
                    .mark_emails_as_deleted(&folder, vec![&internal_id])
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
                    .set_flags(&folder, vec![&envelope.id], &envelope.flags)
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
                    .set_flags(&folder, vec![&envelope.id], &envelope.flags)
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
        let mut local = self.local_builder.build()?;
        let mut remote = self.remote_builder.build().await?;

        loop {
            // wraps in a block to free the lock
            let hunks = {
                let mut lock = self.patch.lock().await;
                lock.pop()
            };

            match hunks {
                None => break,
                Some(hunks) => {
                    for hunk in hunks {
                        trace!("sync runner {} processing envelope hunk: {hunk:?}", self.id);

                        match Self::process_hunk(&mut local, remote.as_mut(), &hunk).await {
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
