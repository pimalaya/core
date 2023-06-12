use log::{trace, warn};
use std::sync::Mutex;

use crate::{
    Backend, BackendBuilder, BackendSyncProgress, BackendSyncProgressEvent, MaildirBackendBuilder,
};

use super::*;

pub struct EnvelopeSyncRunner<'a> {
    pub id: usize,
    pub local_builder: &'a MaildirBackendBuilder,
    pub remote_builder: &'a BackendBuilder,
    pub on_progress: &'a BackendSyncProgress<'a>,
    pub patch: &'a Mutex<Vec<Vec<EnvelopeSyncHunk>>>,
}

impl EnvelopeSyncRunner<'_> {
    pub fn run(&self) -> Result<EnvelopeSyncReport> {
        let mut report = EnvelopeSyncReport::default();
        let mut local = self.local_builder.build()?;
        let mut remote = self.remote_builder.build().map_err(Box::new)?;

        loop {
            match self.patch.try_lock().map(|mut patch| patch.pop()) {
                Err(_) => continue,
                Ok(None) => break,
                Ok(Some(hunks)) => {
                    for hunk in hunks {
                        trace!("sync runner {} processing envelope hunk: {hunk:?}", self.id);

                        let mut process_hunk = |hunk: &EnvelopeSyncHunk| {
                            Ok(match hunk {
                                EnvelopeSyncHunk::GetEnvelopeThenCacheIt(
                                    folder,
                                    internal_id,
                                    Destination::Local,
                                ) => {
                                    let envelope = local
                                        .get_envelope(&folder, &internal_id)
                                        .map_err(Box::new)?;
                                    vec![EnvelopeSyncCacheHunk::InsertEnvelope(
                                        folder.clone(),
                                        envelope.clone(),
                                        Destination::Local,
                                    )]
                                }
                                EnvelopeSyncHunk::GetEnvelopeThenCacheIt(
                                    folder,
                                    internal_id,
                                    Destination::Remote,
                                ) => {
                                    let envelope = remote
                                        .get_envelope(&folder, &internal_id)
                                        .map_err(Box::new)?;
                                    vec![EnvelopeSyncCacheHunk::InsertEnvelope(
                                        folder.clone(),
                                        envelope.clone(),
                                        Destination::Remote,
                                    )]
                                }
                                EnvelopeSyncHunk::CopyEmailThenCacheIt(
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
                                                cache_hunks.push(
                                                    EnvelopeSyncCacheHunk::InsertEnvelope(
                                                        folder.clone(),
                                                        envelope.clone(),
                                                        Destination::Local,
                                                    ),
                                                )
                                            };
                                            local
                                                .preview_emails(&folder, internal_ids)
                                                .map_err(Box::new)?
                                        }
                                        Destination::Remote => {
                                            if *refresh_source_cache {
                                                cache_hunks.push(
                                                    EnvelopeSyncCacheHunk::InsertEnvelope(
                                                        folder.clone(),
                                                        envelope.clone(),
                                                        Destination::Remote,
                                                    ),
                                                )
                                            };
                                            remote
                                                .preview_emails(&folder, internal_ids)
                                                .map_err(Box::new)?
                                        }
                                    };

                                    let emails = emails.to_vec();
                                    let email = emails.first().ok_or_else(|| {
                                        Error::FindEmailError(envelope.id.clone())
                                    })?;

                                    match target {
                                        Destination::Local => {
                                            let internal_id = local
                                                .add_email(&folder, email.raw()?, &envelope.flags)
                                                .map_err(Box::new)?;
                                            let envelope = local
                                                .get_envelope(&folder, &internal_id)
                                                .map_err(Box::new)?;
                                            cache_hunks.push(
                                                EnvelopeSyncCacheHunk::InsertEnvelope(
                                                    folder.clone(),
                                                    envelope.clone(),
                                                    Destination::Local,
                                                ),
                                            );
                                        }
                                        Destination::Remote => {
                                            let internal_id = remote
                                                .add_email(&folder, email.raw()?, &envelope.flags)
                                                .map_err(Box::new)?;
                                            let envelope = remote
                                                .get_envelope(&folder, &internal_id)
                                                .map_err(Box::new)?;
                                            cache_hunks.push(
                                                EnvelopeSyncCacheHunk::InsertEnvelope(
                                                    folder.clone(),
                                                    envelope.clone(),
                                                    Destination::Remote,
                                                ),
                                            );
                                        }
                                    };
                                    cache_hunks
                                }
                                EnvelopeSyncHunk::DeleteCachedEnvelope(
                                    folder,
                                    internal_id,
                                    Destination::Local,
                                ) => {
                                    vec![EnvelopeSyncCacheHunk::DeleteEnvelope(
                                        folder.clone(),
                                        internal_id.clone(),
                                        Destination::Local,
                                    )]
                                }
                                EnvelopeSyncHunk::RemoveEmail(
                                    folder,
                                    internal_id,
                                    Destination::Local,
                                ) => {
                                    local
                                        .mark_emails_as_deleted(&folder, vec![&internal_id])
                                        .map_err(Box::new)?;
                                    vec![]
                                }
                                EnvelopeSyncHunk::DeleteCachedEnvelope(
                                    folder,
                                    internal_id,
                                    Destination::Remote,
                                ) => {
                                    vec![EnvelopeSyncCacheHunk::DeleteEnvelope(
                                        folder.clone(),
                                        internal_id.clone(),
                                        Destination::Remote,
                                    )]
                                }
                                EnvelopeSyncHunk::RemoveEmail(
                                    folder,
                                    internal_id,
                                    Destination::Remote,
                                ) => {
                                    remote
                                        .mark_emails_as_deleted(&folder, vec![&internal_id])
                                        .map_err(Box::new)?;
                                    vec![]
                                }
                                EnvelopeSyncHunk::UpdateCachedFlags(
                                    folder,
                                    envelope,
                                    Destination::Local,
                                ) => {
                                    vec![
                                        EnvelopeSyncCacheHunk::DeleteEnvelope(
                                            folder.clone(),
                                            envelope.id.clone(),
                                            Destination::Local,
                                        ),
                                        EnvelopeSyncCacheHunk::InsertEnvelope(
                                            folder.clone(),
                                            envelope.clone(),
                                            Destination::Local,
                                        ),
                                    ]
                                }
                                EnvelopeSyncHunk::SetFlags(
                                    folder,
                                    envelope,
                                    Destination::Local,
                                ) => {
                                    local
                                        .set_flags(&folder, vec![&envelope.id], &envelope.flags)
                                        .map_err(Box::new)?;
                                    vec![]
                                }
                                EnvelopeSyncHunk::UpdateCachedFlags(
                                    folder,
                                    envelope,
                                    Destination::Remote,
                                ) => {
                                    vec![
                                        EnvelopeSyncCacheHunk::DeleteEnvelope(
                                            folder.clone(),
                                            envelope.id.clone(),
                                            Destination::Remote,
                                        ),
                                        EnvelopeSyncCacheHunk::InsertEnvelope(
                                            folder.clone(),
                                            envelope.clone(),
                                            Destination::Remote,
                                        ),
                                    ]
                                }
                                EnvelopeSyncHunk::SetFlags(
                                    folder,
                                    envelope,
                                    Destination::Remote,
                                ) => {
                                    remote
                                        .set_flags(&folder, vec![&envelope.id], &envelope.flags)
                                        .map_err(Box::new)?;
                                    vec![]
                                }
                            })
                        };

                        match process_hunk(&hunk) {
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
                            .emit(BackendSyncProgressEvent::ProcessEnvelopeHunk(hunk));
                    }
                }
            }
        }

        Ok(report)
    }
}
