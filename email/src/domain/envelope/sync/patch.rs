use log::{error, info, trace, warn};
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use crate::{
    backend::sync::{Source, Target},
    BackendSyncProgress, BackendSyncProgressEvent,
};

use super::*;

pub type Envelopes = HashMap<String, Envelope>;
pub type EnvelopeSyncPatch = HashSet<Vec<EnvelopeSyncHunk>>;

pub struct EnvelopeSyncPatchManager<'a> {
    account_config: &'a AccountConfig,
    local_builder: &'a MaildirBackendBuilder,
    remote_builder: &'a BackendBuilder,
    on_progress: &'a BackendSyncProgress<'a>,
    dry_run: bool,
}

impl<'a> EnvelopeSyncPatchManager<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        local_builder: &'a MaildirBackendBuilder,
        remote_builder: &'a BackendBuilder,
        on_progress: &'a BackendSyncProgress<'a>,
        dry_run: bool,
    ) -> Self {
        Self {
            account_config,
            local_builder,
            remote_builder,
            on_progress,
            dry_run,
        }
    }

    pub fn build_patch(&self, folder: impl ToString) -> Result<EnvelopeSyncPatch> {
        let folder = folder.to_string();
        let account = &self.account_config.name;
        let conn = &mut self.account_config.sync_db_builder()?;
        info!("synchronizing {folder} envelopes of account {account}");

        self.on_progress
            .emit(BackendSyncProgressEvent::GetLocalCachedEnvelopes);

        let mut local = self.local_builder.build()?;
        let mut remote = self.remote_builder.build().map_err(Box::new)?;

        let local_envelopes_cached: Envelopes = HashMap::from_iter(
            Cache::list_local_envelopes(conn, account, &folder)?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );

        trace!("local envelopes cached: {:#?}", local_envelopes_cached);

        self.on_progress
            .emit(BackendSyncProgressEvent::GetLocalEnvelopes);

        let local_envelopes: Envelopes = HashMap::from_iter(
            local
                .list_envelopes(&folder, 0, 0)
                .or_else(|err| {
                    if self.dry_run {
                        Ok(Default::default())
                    } else {
                        Err(Box::new(err))
                    }
                })?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );

        trace!("local envelopes: {:#?}", local_envelopes);

        self.on_progress
            .emit(BackendSyncProgressEvent::GetRemoteCachedEnvelopes);

        let remote_envelopes_cached: Envelopes = HashMap::from_iter(
            Cache::list_remote_envelopes(conn, account, &folder)?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );

        trace!("remote envelopes cached: {:#?}", remote_envelopes_cached);

        self.on_progress
            .emit(BackendSyncProgressEvent::GetRemoteEnvelopes);

        let remote_envelopes: Envelopes = HashMap::from_iter(
            remote
                .list_envelopes(&folder, 0, 0)
                .or_else(|err| {
                    if self.dry_run {
                        Ok(Default::default())
                    } else {
                        Err(Box::new(err))
                    }
                })?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );

        trace!("remote envelopes: {:#?}", remote_envelopes);

        let patch = build_patch(
            &folder,
            local_envelopes_cached,
            local_envelopes,
            remote_envelopes_cached,
            remote_envelopes,
        );

        trace!("envelopes patch: {:#?}", patch);

        self.on_progress
            .emit(BackendSyncProgressEvent::EnvelopesDiffPatchBuilt(
                folder.clone(),
                patch.clone(),
            ));

        Ok(patch)
    }

    pub fn apply_patch(
        &self,
        conn: &mut rusqlite::Connection,
        patch: EnvelopeSyncPatch,
    ) -> Result<EnvelopeSyncReport> {
        let account = &self.account_config.name;
        let mut report = EnvelopeSyncReport::default();

        if self.dry_run {
            info!("dry run enabled, skipping envelopes patch");
            report.patch = patch
                .into_iter()
                .flatten()
                .map(|patch| (patch, None))
                .collect();
        } else {
            let patch = Mutex::new(Vec::from_iter(patch));

            let mut report = (0..16)
                .into_par_iter()
                .map(|id| EnvelopeSyncRunner {
                    id,
                    local_builder: self.local_builder,
                    remote_builder: self.remote_builder,
                    patch: &patch,
                    on_progress: &self.on_progress,
                })
                .filter_map(|runner| match runner.run() {
                    Ok(report) => Some(report),
                    Err(err) => {
                        warn!("error while starting envelope sync runner, skipping it");
                        error!("error while starting envelope sync runner: {err:?}");
                        None
                    }
                })
                .reduce(EnvelopeSyncReport::default, |mut r1, r2| {
                    r1.patch.extend(r2.patch);
                    r1.cache_patch.0.extend(r2.cache_patch.0);
                    r1
                });

            self.on_progress
                .emit(BackendSyncProgressEvent::ProcessEnvelopeCachePatch(
                    report.cache_patch.0.clone(),
                ));

            let mut process_cache_patch = || {
                let tx = conn.transaction()?;
                for hunk in &report.cache_patch.0 {
                    match hunk {
                        EnvelopeSyncCacheHunk::InsertEnvelope(folder, envelope, Target::Local) => {
                            Cache::insert_local_envelope(&tx, account, folder, envelope.clone())?
                        }
                        EnvelopeSyncCacheHunk::InsertEnvelope(folder, envelope, Target::Remote) => {
                            Cache::insert_remote_envelope(&tx, account, folder, envelope.clone())?
                        }
                        EnvelopeSyncCacheHunk::DeleteEnvelope(
                            folder,
                            internal_id,
                            Target::Local,
                        ) => Cache::delete_local_envelope(&tx, account, folder, internal_id)?,
                        EnvelopeSyncCacheHunk::DeleteEnvelope(
                            folder,
                            internal_id,
                            Target::Remote,
                        ) => Cache::delete_remote_envelope(&tx, account, folder, internal_id)?,
                    }
                }
                tx.commit()?;
                Result::Ok(())
            };

            if let Err(err) = process_cache_patch() {
                warn!("error while processing cache patch: {err}");
                report.cache_patch.1 = Some(err);
            }
        }

        trace!("sync report: {:#?}", report);

        Ok(report)
    }
}

pub fn build_patch(
    folder: &str,
    local_cache: Envelopes,
    local: Envelopes,
    remote_cache: Envelopes,
    remote: Envelopes,
) -> EnvelopeSyncPatch {
    let mut patch = EnvelopeSyncPatch::default();
    let mut message_ids = HashSet::new();

    // gather all existing ids found in all envelopes
    message_ids.extend(local_cache.iter().map(|(id, _)| id.as_str()));
    message_ids.extend(local.iter().map(|(id, _)| id.as_str()));
    message_ids.extend(remote_cache.iter().map(|(id, _)| id.as_str()));
    message_ids.extend(remote.iter().map(|(id, _)| id.as_str()));

    // Given the matrix local_cache × local × remote_cache × remote,
    // check every 2⁴ = 16 possibilities:
    for message_id in message_ids {
        let local_cache = local_cache.get(message_id);
        let local = local.get(message_id);
        let remote_cache = remote_cache.get(message_id);
        let remote = remote.get(message_id);

        match (local_cache, local, remote_cache, remote) {
            // 0000
            //
            // The message_id exists nowhere, which cannot happen since
            // message_ides has been built from all envelopes message_id.
            (None, None, None, None) => (),

            // 0001
            //
            // The message_id only exists in the remote side, which means a
            // new email has been added remote side and needs to be
            // cached remote side + copied local side.
            (None, None, None, Some(remote)) => {
                patch.insert(vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    folder.to_string(),
                    remote.clone(),
                    Source::Remote,
                    Target::Local,
                    true,
                )]);
            }

            // 0010
            //
            // The message_id only exists in the remote cache, which means
            // an email is outdated and needs to be removed from the
            // remote cache.
            (None, None, Some(remote_cache), None) => {
                patch.insert(vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    Target::Remote,
                )]);
            }

            // 0011
            //
            // The message_id exists in the remote side but not in the local
            // side, which means there is a conflict. Since we cannot
            // determine which side (local removed or remote added) is
            // the most up-to-date, it is safer to consider the remote
            // added side up-to-date in order not to lose data.
            //
            // TODO: make this behaviour customizable.
            (None, None, Some(remote_cache), Some(remote)) => {
                patch.insert(vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    folder.to_string(),
                    remote.clone(),
                    Source::Remote,
                    Target::Local,
                    false,
                )]);

                if remote_cache.flags != remote.flags {
                    patch.insert(vec![EnvelopeSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: remote.flags.clone(),
                            ..remote_cache.clone()
                        },
                        Target::Remote,
                    )]);
                }
            }

            // 0100
            //
            // The message_id only exists in the local side, which means a
            // new email has been added local side and needs to be
            // added cached local side + added remote sides.
            (None, Some(local), None, None) => {
                patch.insert(vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    folder.to_string(),
                    local.clone(),
                    Source::Local,
                    Target::Remote,
                    true,
                )]);
            }

            // 0101
            //
            // The message_id exists in both local and remote sides, which
            // means a new (same) email has been added both sides and
            // the most recent needs to be kept.
            //
            // NOTE: this case should never happen: new emails
            // internal identifier are unique and should (in theory)
            // never conflict, but we implement this case for the sake
            // of exhaustiveness.
            (None, Some(local), None, Some(remote)) => {
                if local.date > remote.date {
                    patch.insert(vec![
                        EnvelopeSyncHunk::RemoveEmail(
                            folder.to_string(),
                            remote.id.clone(),
                            Target::Remote,
                        ),
                        EnvelopeSyncHunk::CopyEmailThenCacheIt(
                            folder.to_string(),
                            local.clone(),
                            Source::Local,
                            Target::Remote,
                            true,
                        ),
                    ]);
                } else {
                    patch.insert(vec![
                        EnvelopeSyncHunk::RemoveEmail(
                            folder.to_string(),
                            local.id.clone(),
                            Target::Local,
                        ),
                        EnvelopeSyncHunk::CopyEmailThenCacheIt(
                            folder.to_string(),
                            remote.clone(),
                            Source::Remote,
                            Target::Local,
                            true,
                        ),
                    ]);
                }
            }

            // 0110
            //
            // The message_id exists in the local side and in the remote
            // cache side, which means a new (same) email has been
            // added local side but removed remote side. Since we
            // cannot determine which side (local added or remote
            // removed) is the most up-to-date, it is safer to
            // consider the remote added side up-to-date in order not
            // to lose data.
            //
            // TODO: make this behaviour customizable.
            (None, Some(local), Some(remote_cache), None) => {
                patch.insert(vec![
                    EnvelopeSyncHunk::DeleteCachedEnvelope(
                        folder.to_string(),
                        remote_cache.id.clone(),
                        Target::Remote,
                    ),
                    EnvelopeSyncHunk::CopyEmailThenCacheIt(
                        folder.to_string(),
                        local.clone(),
                        Source::Local,
                        Target::Remote,
                        true,
                    ),
                ]);
            }

            // 0111
            //
            // The message_id exists everywhere except in the local cache,
            // which means the local cache misses an email and needs
            // to be updated. Flags also need to be synchronized.
            (None, Some(local), Some(remote_cache), Some(remote)) => {
                patch.insert(vec![EnvelopeSyncHunk::GetEnvelopeThenCacheIt(
                    folder.to_string(),
                    local.id.clone(),
                    Source::Local,
                )]);

                let flags = flag::sync_all(None, Some(local), Some(remote_cache), Some(remote));

                if local.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::SetFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        Target::Local,
                    )]);
                }

                if remote_cache.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote_cache.clone()
                        },
                        Target::Remote,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::SetFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        Target::Remote,
                    )]);
                }
            }

            // 1000
            //
            // The message_id only exists in the local cache, which means
            // the local cache has an outdated email and need to be
            // cleaned.
            (Some(local_cache), None, None, None) => {
                patch.insert(vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )]);
            }

            // 1001
            //
            // The message_id exists in the local cache and in the remote,
            // which means a new (same) email has been removed local
            // side but added remote side. Since we cannot determine
            // which side (local removed or remote added) is the most
            // up-to-date, it is safer to consider the remote added
            // side up-to-date in order not to lose data.
            //
            // TODO: make this behaviour customizable.
            (Some(local_cache), None, None, Some(remote)) => {
                patch.insert(vec![
                    EnvelopeSyncHunk::DeleteCachedEnvelope(
                        folder.to_string(),
                        local_cache.id.clone(),
                        Target::Local,
                    ),
                    EnvelopeSyncHunk::CopyEmailThenCacheIt(
                        folder.to_string(),
                        remote.clone(),
                        Source::Remote,
                        Target::Local,
                        true,
                    ),
                ]);
            }

            // 1010
            //
            // The message_id only exists in both caches, which means caches
            // have an outdated email and need to be cleaned up.
            (Some(local_cache), None, Some(remote_cache), None) => patch.extend([
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )],
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    Target::Remote,
                )],
            ]),

            // 1011
            //
            // The message_id exists everywhere except in local side, which
            // means an email has been removed local side and needs to
            // be removed everywhere else.
            (Some(local_cache), None, Some(remote_cache), Some(remote)) => patch.extend([
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )],
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    Target::Remote,
                )],
                vec![EnvelopeSyncHunk::RemoveEmail(
                    folder.to_string(),
                    remote.id.clone(),
                    Target::Remote,
                )],
            ]),

            // 1100
            //
            // The message_id exists in local side but not in remote side,
            // which means there is a conflict. Since we cannot
            // determine which side (local updated or remote removed)
            // is the most up-to-date, it is safer to consider the
            // local updated side up-to-date in order not to lose
            // data.
            //
            // TODO: make this behaviour customizable.
            (Some(local_cache), Some(local), None, None) => {
                patch.insert(vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    folder.to_string(),
                    local.clone(),
                    Source::Local,
                    Target::Remote,
                    false,
                )]);

                if local_cache.flags != local.flags {
                    patch.insert(vec![EnvelopeSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: local.flags.clone(),
                            ..local_cache.clone()
                        },
                        Target::Local,
                    )]);
                }
            }

            // 1101
            //
            // The message_id exists everywhere except in remote cache side,
            // which means an email is missing remote cache side and
            // needs to be updated. Flags also need to be
            // synchronized.
            (Some(local_cache), Some(local), None, Some(remote)) => {
                let flags = flag::sync_all(Some(local_cache), Some(local), None, Some(remote));

                if local_cache.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local_cache.clone()
                        },
                        Target::Local,
                    )]);
                }

                if local.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::SetFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        Target::Local,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::SetFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        Target::Remote,
                    )]);
                }

                patch.insert(vec![EnvelopeSyncHunk::GetEnvelopeThenCacheIt(
                    folder.to_string(),
                    remote.id.clone(),
                    Source::Remote,
                )]);
            }

            // 1110
            //
            // The message_id exists everywhere except in remote side, which
            // means an email has been removed remote side and needs
            // to be removed everywhere else.
            (Some(local_cache), Some(local), Some(remote_cache), None) => patch.extend([
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )],
                vec![EnvelopeSyncHunk::RemoveEmail(
                    folder.to_string(),
                    local.id.clone(),
                    Target::Local,
                )],
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    Target::Remote,
                )],
            ]),

            // 1111
            //
            // The message_id exists everywhere, which means all flags need
            // to be synchronized.
            (Some(local_cache), Some(local), Some(remote_cache), Some(remote)) => {
                let flags = flag::sync_all(
                    Some(local_cache),
                    Some(local),
                    Some(remote_cache),
                    Some(remote),
                );

                if local_cache.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local_cache.clone()
                        },
                        Target::Local,
                    )]);
                }

                if local.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::SetFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        Target::Local,
                    )]);
                }

                if remote_cache.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote_cache.clone()
                        },
                        Target::Remote,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EnvelopeSyncHunk::SetFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        Target::Remote,
                    )]);
                }
            }
        }
    }

    patch
}

#[cfg(test)]
mod tests {
    use crate::{
        backend::sync::{Source, Target},
        Envelope, Flag, Flags,
    };

    use super::{EnvelopeSyncHunk, EnvelopeSyncPatch, Envelopes};

    #[test]
    fn build_patch_0000() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::default()
        );
    }

    #[test]
    fn build_patch_0001() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::default();
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                "inbox".into(),
                Envelope {
                    id: "remote-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                Source::Remote,
                Target::Local,
                true,
            )]]),
        );
    }

    #[test]
    fn build_patch_0010() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                "inbox".into(),
                "remote-cache-id".into(),
                Target::Remote
            )]]),
        );
    }

    #[test]
    fn build_patch_0011_same_flags() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                "inbox".into(),
                Envelope {
                    id: "remote-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                Target::Remote,
                Target::Local,
                false,
            )]]),
        );
    }

    #[test]
    fn build_patch_0011_different_flags() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen replied".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen flagged deleted".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([
                vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    "inbox".into(),
                    Envelope {
                        id: "remote-id".into(),
                        flags: "seen flagged deleted".into(),
                        ..Envelope::default()
                    },
                    Target::Remote,
                    Target::Local,
                    false,
                )],
                vec![EnvelopeSyncHunk::UpdateCachedFlags(
                    "inbox".into(),
                    Envelope {
                        id: "remote-cache-id".into(),
                        flags: Flags::from_iter([Flag::Seen, Flag::Flagged, Flag::Deleted]),
                        ..Envelope::default()
                    },
                    Target::Remote,
                )]
            ])
        );
    }

    #[test]
    fn build_patch_0100() {
        let local_cache = Envelopes::default();
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                "inbox".into(),
                Envelope {
                    id: "local-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                Target::Local,
                Target::Remote,
                true,
            )]]),
        );
    }

    #[test]
    fn build_patch_0101() {
        let local_cache = Envelopes::default();
        let local = Envelopes::from_iter([
            (
                "message_id-1".into(),
                Envelope {
                    id: "local-id-1".into(),
                    flags: "seen".into(),
                    date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-2".into(),
                Envelope {
                    id: "local-id-2".into(),
                    flags: "seen".into(),
                    date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-3".into(),
                Envelope {
                    id: "local-id-3".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-4".into(),
                Envelope {
                    id: "local-id-4".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-5".into(),
                Envelope {
                    id: "local-id-5".into(),
                    flags: "seen".into(),
                    date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                    ..Envelope::default()
                },
            ),
        ]);
        let remote_cache = Envelopes::default();
        let remote = Envelopes::from_iter([
            (
                "message_id-1".into(),
                Envelope {
                    id: "remote-id-1".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-2".into(),
                Envelope {
                    id: "remote-id-2".into(),
                    flags: "seen".into(),
                    date: "2021-01-01T00:00:00-00:00".parse().unwrap(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-3".into(),
                Envelope {
                    id: "remote-id-3".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-4".into(),
                Envelope {
                    id: "remote-id-4".into(),
                    flags: "seen".into(),
                    date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                    ..Envelope::default()
                },
            ),
            (
                "message_id-5".into(),
                Envelope {
                    id: "remote-id-5".into(),
                    flags: "seen".into(),
                    date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                    ..Envelope::default()
                },
            ),
        ]);

        let patch = super::build_patch("inbox", local_cache, local, remote_cache, remote)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(patch.len(), 10);
        assert!(patch.contains(&EnvelopeSyncHunk::RemoveEmail(
            "inbox".into(),
            "remote-id-1".into(),
            Target::Remote
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::CopyEmailThenCacheIt(
            "inbox".into(),
            Envelope {
                id: "local-id-1".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            Target::Local,
            Target::Remote,
            true,
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::RemoveEmail(
            "inbox".into(),
            "remote-id-2".into(),
            Target::Remote
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::CopyEmailThenCacheIt(
            "inbox".into(),
            Envelope {
                id: "local-id-2".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            Target::Local,
            Target::Remote,
            true,
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::RemoveEmail(
            "inbox".into(),
            "local-id-3".into(),
            Target::Local
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::CopyEmailThenCacheIt(
            "inbox".into(),
            Envelope {
                id: "remote-id-3".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
            Target::Remote,
            Target::Local,
            true,
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::RemoveEmail(
            "inbox".into(),
            "local-id-4".into(),
            Target::Local
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::CopyEmailThenCacheIt(
            "inbox".into(),
            Envelope {
                id: "remote-id-4".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            Target::Remote,
            Target::Local,
            true,
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::RemoveEmail(
            "inbox".into(),
            "local-id-5".into(),
            Target::Local
        )));
        assert!(patch.contains(&EnvelopeSyncHunk::CopyEmailThenCacheIt(
            "inbox".into(),
            Envelope {
                id: "remote-id-5".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            Target::Remote,
            Target::Local,
            true,
        )));
    }

    #[test]
    fn build_patch_0110() {
        let local_cache = Envelopes::default();
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "flagged".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![
                EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "remote-id".into(),
                    Target::Remote
                ),
                EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    "inbox".into(),
                    Envelope {
                        id: "local-id".into(),
                        flags: "seen".into(),
                        ..Envelope::default()
                    },
                    Target::Local,
                    Target::Remote,
                    true,
                )
            ]]),
        );
    }

    #[test]
    fn build_patch_0111() {
        let local_cache = Envelopes::default();
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::GetEnvelopeThenCacheIt(
                "inbox".into(),
                "local-id".into(),
                Target::Local,
            )]])
        );
    }

    #[test]
    fn build_patch_1000() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::default();
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                "inbox".into(),
                "local-cache-id".into(),
                Target::Local
            )]])
        );
    }

    #[test]
    fn build_patch_1001() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::default();
        let remote_cache = Envelopes::default();
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![
                EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local
                ),
                EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    "inbox".into(),
                    Envelope {
                        id: "remote-id".into(),
                        flags: "seen".into(),
                        ..Envelope::default()
                    },
                    Target::Remote,
                    Target::Local,
                    true,
                ),
            ]])
        );
    }

    #[test]
    fn build_patch_1010() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::default();
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local
                )],
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    Target::Remote
                )],
            ])
        );
    }

    #[test]
    fn build_patch_1011() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::default();
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local,
                )],
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    Target::Remote,
                )],
                vec![EnvelopeSyncHunk::RemoveEmail(
                    "inbox".into(),
                    "remote-id".into(),
                    Target::Remote
                )],
            ])
        );
    }

    #[test]
    fn build_patch_1100_same_flags() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                "inbox".into(),
                Envelope {
                    id: "local-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                Target::Local,
                Target::Remote,
                false,
            )]])
        );
    }

    #[test]
    fn build_patch_1100_different_flags() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "flagged".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([
                vec![EnvelopeSyncHunk::CopyEmailThenCacheIt(
                    "inbox".into(),
                    Envelope {
                        id: "local-id".into(),
                        flags: "flagged".into(),
                        ..Envelope::default()
                    },
                    Target::Local,
                    Target::Remote,
                    false,
                )],
                vec![EnvelopeSyncHunk::UpdateCachedFlags(
                    "inbox".into(),
                    Envelope {
                        id: "local-cache-id".into(),
                        flags: Flags::from_iter([Flag::Flagged]),
                        ..Envelope::default()
                    },
                    Target::Local,
                )]
            ])
        );
    }

    #[test]
    fn build_patch_1101() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::default();
        let remote = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([vec![EnvelopeSyncHunk::GetEnvelopeThenCacheIt(
                "inbox".into(),
                "remote-id".into(),
                Target::Remote,
            )]]),
        );
    }

    #[test]
    fn build_patch_1110() {
        let local_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let local = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "local-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote_cache = Envelopes::from_iter([(
            "message_id".into(),
            Envelope {
                id: "remote-cache-id".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
        )]);
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EnvelopeSyncPatch::from_iter([
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local,
                )],
                vec![EnvelopeSyncHunk::RemoveEmail(
                    "inbox".into(),
                    "local-id".into(),
                    Target::Local
                )],
                vec![EnvelopeSyncHunk::DeleteCachedEnvelope(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    Target::Remote,
                )],
            ])
        );
    }
}
