use log::{trace, warn};
use std::sync::Mutex;

use crate::{
    Backend, BackendBuilder, BackendSyncProgress, BackendSyncProgressEvent, MaildirBackendBuilder,
};

use super::*;

pub struct EmailSyncRunner<'a> {
    pub id: usize,
    pub local_builder: &'a MaildirBackendBuilder,
    pub remote_builder: &'a BackendBuilder,
    pub on_progress: &'a BackendSyncProgress<'a>,
    pub patch: &'a Mutex<Vec<Vec<EmailSyncHunk>>>,
}

impl EmailSyncRunner<'_> {
    pub fn run(&self) -> Result<EmailSyncReport> {
        let mut report = EmailSyncReport::default();
        let mut local = self.local_builder.build()?;
        let mut remote = self.remote_builder.build()?;

        loop {
            match self.patch.try_lock().map(|mut patch| patch.pop()) {
                Err(_) => continue,
                Ok(None) => break,
                Ok(Some(hunks)) => {
                    for hunk in hunks {
                        trace!("sync runner {} processing envelope hunk: {hunk:?}", self.id);

                        let mut process_hunk = |hunk: &EmailSyncHunk| {
                            Ok(match hunk {
                                EmailSyncHunk::GetThenCache(
                                    folder,
                                    internal_id,
                                    Destination::Local,
                                ) => {
                                    let envelope = local.get_envelope(&folder, &internal_id)?;
                                    vec![EmailSyncCacheHunk::Insert(
                                        folder.clone(),
                                        envelope.clone(),
                                        Destination::Local,
                                    )]
                                }
                                EmailSyncHunk::GetThenCache(
                                    folder,
                                    internal_id,
                                    Destination::Remote,
                                ) => {
                                    let envelope = remote.get_envelope(&folder, &internal_id)?;
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
                                            local.preview_emails(&folder, internal_ids)?
                                        }
                                        Destination::Remote => {
                                            if *refresh_source_cache {
                                                cache_hunks.push(EmailSyncCacheHunk::Insert(
                                                    folder.clone(),
                                                    envelope.clone(),
                                                    Destination::Remote,
                                                ))
                                            };
                                            remote.preview_emails(&folder, internal_ids)?
                                        }
                                    };

                                    let emails = emails.to_vec();
                                    let email = emails.first().ok_or_else(|| {
                                        Error::FindEmailError(envelope.id.clone())
                                    })?;

                                    match target {
                                        Destination::Local => {
                                            let internal_id = local.add_email(
                                                &folder,
                                                email.raw()?,
                                                &envelope.flags,
                                            )?;
                                            let envelope =
                                                local.get_envelope(&folder, &internal_id)?;
                                            cache_hunks.push(EmailSyncCacheHunk::Insert(
                                                folder.clone(),
                                                envelope.clone(),
                                                Destination::Local,
                                            ));
                                        }
                                        Destination::Remote => {
                                            let internal_id = remote.add_email(
                                                &folder,
                                                email.raw()?,
                                                &envelope.flags,
                                            )?;
                                            let envelope =
                                                remote.get_envelope(&folder, &internal_id)?;
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
                                    local.mark_emails_as_deleted(&folder, vec![&internal_id])?;
                                    vec![]
                                }
                                EmailSyncHunk::Uncache(
                                    folder,
                                    internal_id,
                                    Destination::Remote,
                                ) => {
                                    vec![EmailSyncCacheHunk::Delete(
                                        folder.clone(),
                                        internal_id.clone(),
                                        Destination::Remote,
                                    )]
                                }
                                EmailSyncHunk::Delete(folder, internal_id, Destination::Remote) => {
                                    remote.mark_emails_as_deleted(&folder, vec![&internal_id])?;
                                    vec![]
                                }
                                EmailSyncHunk::UpdateCachedFlags(
                                    folder,
                                    envelope,
                                    Destination::Local,
                                ) => {
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
                                EmailSyncHunk::UpdateFlags(
                                    folder,
                                    envelope,
                                    Destination::Local,
                                ) => {
                                    local.set_flags(
                                        &folder,
                                        vec![&envelope.id],
                                        &envelope.flags,
                                    )?;
                                    vec![]
                                }
                                EmailSyncHunk::UpdateCachedFlags(
                                    folder,
                                    envelope,
                                    Destination::Remote,
                                ) => {
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
                                EmailSyncHunk::UpdateFlags(
                                    folder,
                                    envelope,
                                    Destination::Remote,
                                ) => {
                                    remote.set_flags(
                                        &folder,
                                        vec![&envelope.id],
                                        &envelope.flags,
                                    )?;
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
                            .emit(BackendSyncProgressEvent::ApplyEnvelopeHunk(hunk));
                    }
                }
            }
        }

        Ok(report)
    }
}
