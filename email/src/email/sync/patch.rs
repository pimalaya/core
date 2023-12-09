//! Module dedicated to email synchronization patch.
//!
//! The core structure of the module is the [`EmailSyncPatch`], which
//! represents a list of changes (hunks).
//!
//! You also have access to a [`EmailSyncPatchManager`] which helps
//! you to build and to apply an email patch.

use futures::{lock::Mutex, stream, StreamExt};
use log::{debug, info};
use rusqlite::Connection;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    account::sync::{
        AccountSyncProgress, AccountSyncProgressEvent, LocalBackendBuilder, Source, Target,
    },
    backend::BackendContextBuilder,
    flag, Result,
};

use super::*;

/// Alias for a envelope hash map where the key is its identifier.
pub type Envelopes = HashMap<String, Envelope>;

/// An email synchronization patch is just a list of email
/// synchronization hunks (changes).
pub type EmailSyncPatch = HashSet<Vec<EmailSyncHunk>>;

/// A email synchronization cache patch is just a list of email
/// synchronization cache hunks (changes).
pub type EmailSyncCachePatch = Vec<EmailSyncCacheHunk>;

/// The email synchronization patch manager.
///
/// This structure helps you to build a patch and to apply it.
pub struct EmailSyncPatchManager<'a, B: BackendContextBuilder> {
    account_config: &'a AccountConfig,
    local_builder: LocalBackendBuilder,
    remote_builder: BackendBuilder<B>,
    on_progress: AccountSyncProgress,
    dry_run: bool,
}

impl<'a, B: BackendContextBuilder + 'static> EmailSyncPatchManager<'a, B> {
    /// Creates a new email synchronization patch manager.
    pub fn new(
        account_config: &'a AccountConfig,
        local_builder: LocalBackendBuilder,
        remote_builder: BackendBuilder<B>,
        on_progress: AccountSyncProgress,
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

    /// Builds the email synchronization patch.
    pub async fn build_patch(&self, folder: impl ToString) -> Result<EmailSyncPatch> {
        info!("building envelope sync patch");

        let folder = folder.to_string();
        let account = &self.account_config.name;
        let conn = &mut self.account_config.sync_db_builder()?;

        self.on_progress
            .emit(AccountSyncProgressEvent::GetLocalCachedEnvelopes);

        let local = self.local_builder.clone().build().await?;
        let remote = self.remote_builder.clone().build().await?;

        debug!("getting local cached envelopes");
        let local_envelopes_cached: Envelopes = HashMap::from_iter(
            EmailSyncCache::list_local_envelopes(conn, account, &folder)?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );
        debug!("{local_envelopes_cached:#?}");

        self.on_progress
            .emit(AccountSyncProgressEvent::GetLocalEnvelopes);

        debug!("getting local envelopes");
        let local_envelopes: Envelopes = HashMap::from_iter(
            local
                .list_envelopes(&folder, 0, 0)
                .await
                .or_else(|err| {
                    if self.dry_run {
                        Ok(Default::default())
                    } else {
                        Err(err)
                    }
                })?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );
        debug!("{local_envelopes:#?}");

        self.on_progress
            .emit(AccountSyncProgressEvent::GetRemoteCachedEnvelopes);

        debug!("getting remote cached envelopes");
        let remote_envelopes_cached: Envelopes = HashMap::from_iter(
            EmailSyncCache::list_remote_envelopes(conn, account, &folder)?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );
        debug!("{remote_envelopes_cached:#?}");

        self.on_progress
            .emit(AccountSyncProgressEvent::GetRemoteEnvelopes);

        debug!("getting remote envelopes");
        let remote_envelopes: Envelopes = HashMap::from_iter(
            remote
                .list_envelopes(&folder, 0, 0)
                .await
                .or_else(|err| {
                    if self.dry_run {
                        Ok(Default::default())
                    } else {
                        Err(err)
                    }
                })?
                .iter()
                .map(|envelope| (envelope.message_id.clone(), envelope.clone())),
        );
        debug!("{remote_envelopes:#?}");

        debug!("building envelopes sync patch");
        let patch = build_patch(
            &folder,
            local_envelopes_cached,
            local_envelopes,
            remote_envelopes_cached,
            remote_envelopes,
        );
        debug!("{patch:#?}");

        self.on_progress
            .emit(AccountSyncProgressEvent::EnvelopePatchBuilt(
                folder.clone(),
                patch.clone(),
            ));

        Ok(patch)
    }

    /// Applies all the email synchronization patch built from
    /// `build_patch()`.
    ///
    /// Returns an email synchronization report.
    pub async fn apply_patch(
        &self,
        conn: &mut Connection,
        patch: EmailSyncPatch,
    ) -> Result<EmailSyncReport> {
        info!("applying envelope sync patch");

        let account = &self.account_config.name;
        let mut report = EmailSyncReport::default();

        if self.dry_run {
            debug!("dry run enabled, skipping patch");
            report.patch = patch
                .into_iter()
                .flatten()
                .map(|patch| (patch, None))
                .collect();
        } else {
            let patch = Arc::new(Mutex::new(Vec::from_iter(patch)));

            debug!("starting email sync runners");

            report = stream::iter(0..16)
                .map(|id| EmailSyncRunner {
                    id,
                    local_builder: self.local_builder.clone(),
                    remote_builder: self.remote_builder.clone(),
                    patch: patch.clone(),
                    on_progress: self.on_progress.clone(),
                })
                .map(|runner| {
                    tokio::spawn(async move {
                        match runner.run().await {
                            Ok(report) => Some(report),
                            Err(err) => {
                                debug!("error while starting email sync runner: {err}");
                                debug!("{err:?}");
                                None
                            }
                        }
                    })
                })
                .buffer_unordered(16)
                .filter_map(|report| async {
                    match report {
                        Ok(Some(report)) => Some(report),
                        _ => None,
                    }
                })
                .fold(EmailSyncReport::default(), |mut r1, r2| async {
                    r1.patch.extend(r2.patch);
                    r1.cache_patch.0.extend(r2.cache_patch.0);
                    r1
                })
                .await;

            self.on_progress
                .emit(AccountSyncProgressEvent::ApplyEnvelopeCachePatch(
                    report.cache_patch.0.clone(),
                ));

            let mut process_cache_patch = || {
                let tx = conn.transaction()?;
                for hunk in &report.cache_patch.0 {
                    match hunk {
                        EmailSyncCacheHunk::Insert(folder, envelope, Target::Local) => {
                            EmailSyncCache::insert_local_envelope(
                                &tx,
                                account,
                                folder,
                                envelope.clone(),
                            )?
                        }
                        EmailSyncCacheHunk::Insert(folder, envelope, Target::Remote) => {
                            EmailSyncCache::insert_remote_envelope(
                                &tx,
                                account,
                                folder,
                                envelope.clone(),
                            )?
                        }
                        EmailSyncCacheHunk::Delete(folder, internal_id, Target::Local) => {
                            EmailSyncCache::delete_local_envelope(
                                &tx,
                                account,
                                folder,
                                internal_id,
                            )?
                        }
                        EmailSyncCacheHunk::Delete(folder, internal_id, Target::Remote) => {
                            EmailSyncCache::delete_remote_envelope(
                                &tx,
                                account,
                                folder,
                                internal_id,
                            )?
                        }
                    }
                }
                tx.commit()?;
                Result::Ok(())
            };

            if let Err(err) = process_cache_patch() {
                debug!("error while applying envelope cache patch: {err}");
                debug!("{err:?}");
                report.cache_patch.1 = Some(err);
            }
        }
        debug!("{report:#?}");

        Ok(report)
    }
}

/// Email synchronization patch builder.
///
/// Contains the core algorithm of the email synchronization. It has
/// been exported in a dedicated function so that it can be easily
/// tested.
pub fn build_patch(
    folder: &str,
    local_cache: Envelopes,
    local: Envelopes,
    remote_cache: Envelopes,
    remote: Envelopes,
) -> EmailSyncPatch {
    let mut patch = EmailSyncPatch::default();
    let mut message_ids = HashSet::new();

    // gather all existing ids found in all envelopes
    message_ids.extend(local_cache.iter().map(|(id, _)| id.as_str()));
    message_ids.extend(local.iter().map(|(id, _)| id.as_str()));
    message_ids.extend(remote_cache.iter().map(|(id, _)| id.as_str()));
    message_ids.extend(remote.iter().map(|(id, _)| id.as_str()));

    // Given the matrice local_cache × local × remote_cache × remote,
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
                patch.insert(vec![EmailSyncHunk::CopyThenCache(
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
                patch.insert(vec![EmailSyncHunk::Uncache(
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
                patch.insert(vec![EmailSyncHunk::CopyThenCache(
                    folder.to_string(),
                    remote.clone(),
                    Source::Remote,
                    Target::Local,
                    false,
                )]);

                if remote_cache.flags != remote.flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
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
                patch.insert(vec![EmailSyncHunk::CopyThenCache(
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
                        EmailSyncHunk::Delete(
                            folder.to_string(),
                            remote.id.clone(),
                            Target::Remote,
                        ),
                        EmailSyncHunk::CopyThenCache(
                            folder.to_string(),
                            local.clone(),
                            Source::Local,
                            Target::Remote,
                            true,
                        ),
                    ]);
                } else {
                    patch.insert(vec![
                        EmailSyncHunk::Delete(folder.to_string(), local.id.clone(), Target::Local),
                        EmailSyncHunk::CopyThenCache(
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
                    EmailSyncHunk::Uncache(
                        folder.to_string(),
                        remote_cache.id.clone(),
                        Target::Remote,
                    ),
                    EmailSyncHunk::CopyThenCache(
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
                patch.insert(vec![EmailSyncHunk::GetThenCache(
                    folder.to_string(),
                    local.id.clone(),
                    Source::Local,
                )]);

                let flags = flag::sync(
                    None,
                    Some(&local.flags),
                    Some(&remote_cache.flags),
                    Some(&remote.flags),
                );

                if local.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        Target::Local,
                    )]);
                }

                if remote_cache.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote_cache.clone()
                        },
                        Target::Remote,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
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
                patch.insert(vec![EmailSyncHunk::Uncache(
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
                    EmailSyncHunk::Uncache(
                        folder.to_string(),
                        local_cache.id.clone(),
                        Target::Local,
                    ),
                    EmailSyncHunk::CopyThenCache(
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
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )],
                vec![EmailSyncHunk::Uncache(
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
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )],
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    Target::Remote,
                )],
                vec![EmailSyncHunk::Delete(
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
                patch.insert(vec![EmailSyncHunk::CopyThenCache(
                    folder.to_string(),
                    local.clone(),
                    Source::Local,
                    Target::Remote,
                    false,
                )]);

                if local_cache.flags != local.flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
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
                let flags = flag::sync(
                    Some(&local_cache.flags),
                    Some(&local.flags),
                    None,
                    Some(&remote.flags),
                );

                if local_cache.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local_cache.clone()
                        },
                        Target::Local,
                    )]);
                }

                if local.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        Target::Local,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        Target::Remote,
                    )]);
                }

                patch.insert(vec![EmailSyncHunk::GetThenCache(
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
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    local_cache.id.clone(),
                    Target::Local,
                )],
                vec![EmailSyncHunk::Delete(
                    folder.to_string(),
                    local.id.clone(),
                    Target::Local,
                )],
                vec![EmailSyncHunk::Uncache(
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
                let flags = flag::sync(
                    Some(&local_cache.flags),
                    Some(&local.flags),
                    Some(&remote_cache.flags),
                    Some(&remote.flags),
                );

                if local_cache.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local_cache.clone()
                        },
                        Target::Local,
                    )]);
                }

                if local.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        Target::Local,
                    )]);
                }

                if remote_cache.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote_cache.clone()
                        },
                        Target::Remote,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
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
        account::sync::{Source, Target},
        envelope::Envelope,
        flag::{Flag, Flags},
    };

    use super::{EmailSyncHunk, EmailSyncPatch, Envelopes};

    #[test]
    fn build_patch_0000() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build_patch("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::default()
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::Uncache(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
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
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::CopyThenCache(
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
                vec![EmailSyncHunk::UpdateCachedFlags(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
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
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "remote-id-1".into(),
            Target::Remote
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
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
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "remote-id-2".into(),
            Target::Remote
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
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
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "local-id-3".into(),
            Target::Local
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
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
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "local-id-4".into(),
            Target::Local
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
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
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "local-id-5".into(),
            Target::Local
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
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
            EmailSyncPatch::from_iter([vec![
                EmailSyncHunk::Uncache("inbox".into(), "remote-id".into(), Target::Remote),
                EmailSyncHunk::CopyThenCache(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::GetThenCache(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::Uncache(
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
            EmailSyncPatch::from_iter([vec![
                EmailSyncHunk::Uncache("inbox".into(), "local-cache-id".into(), Target::Local),
                EmailSyncHunk::CopyThenCache(
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
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local
                )],
                vec![EmailSyncHunk::Uncache(
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
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local,
                )],
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    Target::Remote,
                )],
                vec![EmailSyncHunk::Delete(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
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
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::CopyThenCache(
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
                vec![EmailSyncHunk::UpdateCachedFlags(
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
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::GetThenCache(
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
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    Target::Local,
                )],
                vec![EmailSyncHunk::Delete(
                    "inbox".into(),
                    "local-id".into(),
                    Target::Local
                )],
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    Target::Remote,
                )],
            ])
        );
    }
}
