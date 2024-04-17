//! # Email sync patch
//!
//! Module dedicated to email synchronization patch. The main
//! structure of the module is the [`EmailSyncPatch`], which
//! represents a list of changes (hunks).

use std::collections::{HashMap, HashSet};

use super::*;
use crate::flag;

/// Alias for an envelope hash map where the key is its identifier.
pub type Envelopes = HashMap<String, Envelope>;

/// An email synchronization patch is just a list of email
/// synchronization hunks (changes).
// TODO: remove HashSet
pub type EmailSyncPatch = HashSet<Vec<EmailSyncHunk>>;

/// Email synchronization patch builder.
///
/// Contains the core algorithm of the email synchronization. It has
/// been exported in a dedicated function so that it can be easily
/// tested.
pub fn build(
    folder: impl ToString,
    left_cached: Envelopes,
    left: Envelopes,
    right_cached: Envelopes,
    right: Envelopes,
) -> EmailSyncPatch {
    let mut patch = EmailSyncPatch::default();
    let mut message_ids = HashSet::new();

    // gather all existing ids found in all envelopes
    message_ids.extend(left_cached.keys().map(|id| id.as_str()));
    message_ids.extend(left.keys().map(|id| id.as_str()));
    message_ids.extend(right_cached.keys().map(|id| id.as_str()));
    message_ids.extend(right.keys().map(|id| id.as_str()));

    // Given the matrice local_cache × local × remote_cache × remote,
    // check every 2⁴ = 16 possibilities:
    for message_id in message_ids {
        let local_cache = left_cached.get(message_id);
        let local = left.get(message_id);
        let remote_cache = right_cached.get(message_id);
        let remote = right.get(message_id);

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
                    SyncDestination::Right,
                    SyncDestination::Left,
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
                    SyncDestination::Right,
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
                    SyncDestination::Right,
                    SyncDestination::Left,
                    false,
                )]);

                if remote_cache.flags != remote.flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: remote.flags.clone(),
                            ..remote_cache.clone()
                        },
                        SyncDestination::Right,
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
                    SyncDestination::Left,
                    SyncDestination::Right,
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
                            SyncDestination::Right,
                        ),
                        EmailSyncHunk::CopyThenCache(
                            folder.to_string(),
                            local.clone(),
                            SyncDestination::Left,
                            SyncDestination::Right,
                            true,
                        ),
                    ]);
                } else {
                    patch.insert(vec![
                        EmailSyncHunk::Delete(
                            folder.to_string(),
                            local.id.clone(),
                            SyncDestination::Left,
                        ),
                        EmailSyncHunk::CopyThenCache(
                            folder.to_string(),
                            remote.clone(),
                            SyncDestination::Right,
                            SyncDestination::Left,
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
                        SyncDestination::Right,
                    ),
                    EmailSyncHunk::CopyThenCache(
                        folder.to_string(),
                        local.clone(),
                        SyncDestination::Left,
                        SyncDestination::Right,
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
                    SyncDestination::Left,
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
                        SyncDestination::Left,
                    )]);
                }

                if remote_cache.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote_cache.clone()
                        },
                        SyncDestination::Right,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        SyncDestination::Right,
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
                    SyncDestination::Left,
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
                        SyncDestination::Left,
                    ),
                    EmailSyncHunk::CopyThenCache(
                        folder.to_string(),
                        remote.clone(),
                        SyncDestination::Right,
                        SyncDestination::Left,
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
                    SyncDestination::Left,
                )],
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    SyncDestination::Right,
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
                    SyncDestination::Left,
                )],
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    SyncDestination::Right,
                )],
                vec![EmailSyncHunk::Delete(
                    folder.to_string(),
                    remote.id.clone(),
                    SyncDestination::Right,
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
                    SyncDestination::Left,
                    SyncDestination::Right,
                    false,
                )]);

                if local_cache.flags != local.flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: local.flags.clone(),
                            ..local_cache.clone()
                        },
                        SyncDestination::Left,
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
                        SyncDestination::Left,
                    )]);
                }

                if local.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        SyncDestination::Left,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        SyncDestination::Right,
                    )]);
                }

                patch.insert(vec![EmailSyncHunk::GetThenCache(
                    folder.to_string(),
                    remote.id.clone(),
                    SyncDestination::Right,
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
                    SyncDestination::Left,
                )],
                vec![EmailSyncHunk::Delete(
                    folder.to_string(),
                    local.id.clone(),
                    SyncDestination::Left,
                )],
                vec![EmailSyncHunk::Uncache(
                    folder.to_string(),
                    remote_cache.id.clone(),
                    SyncDestination::Right,
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
                        SyncDestination::Left,
                    )]);
                }

                if local.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..local.clone()
                        },
                        SyncDestination::Left,
                    )]);
                }

                if remote_cache.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateCachedFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote_cache.clone()
                        },
                        SyncDestination::Right,
                    )]);
                }

                if remote.flags != flags {
                    patch.insert(vec![EmailSyncHunk::UpdateFlags(
                        folder.to_string(),
                        Envelope {
                            flags: flags.clone(),
                            ..remote.clone()
                        },
                        SyncDestination::Right,
                    )]);
                }
            }
        }
    }

    patch
}

#[cfg(test)]
mod tests {
    use super::{EmailSyncHunk, EmailSyncPatch, Envelopes};
    use crate::{
        envelope::Envelope,
        flag::{Flag, Flags},
        sync::SyncDestination,
    };

    #[test]
    fn build_patch_0000() {
        let local_cache = Envelopes::default();
        let local = Envelopes::default();
        let remote_cache = Envelopes::default();
        let remote = Envelopes::default();

        assert_eq!(
            super::build("inbox", local_cache, local, remote_cache, remote),
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
                "inbox".into(),
                Envelope {
                    id: "remote-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                SyncDestination::Right,
                SyncDestination::Left,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::Uncache(
                "inbox".into(),
                "remote-cache-id".into(),
                SyncDestination::Right
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
                "inbox".into(),
                Envelope {
                    id: "remote-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                SyncDestination::Right,
                SyncDestination::Left,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::CopyThenCache(
                    "inbox".into(),
                    Envelope {
                        id: "remote-id".into(),
                        flags: "seen flagged deleted".into(),
                        ..Envelope::default()
                    },
                    SyncDestination::Right,
                    SyncDestination::Left,
                    false,
                )],
                vec![EmailSyncHunk::UpdateCachedFlags(
                    "inbox".into(),
                    Envelope {
                        id: "remote-cache-id".into(),
                        flags: Flags::from_iter([Flag::Seen, Flag::Flagged, Flag::Deleted]),
                        ..Envelope::default()
                    },
                    SyncDestination::Right,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
                "inbox".into(),
                Envelope {
                    id: "local-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                SyncDestination::Left,
                SyncDestination::Right,
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

        let patch = super::build("inbox", local_cache, local, remote_cache, remote)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        assert_eq!(patch.len(), 10);
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "remote-id-1".into(),
            SyncDestination::Right
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
            "inbox".into(),
            Envelope {
                id: "local-id-1".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            SyncDestination::Left,
            SyncDestination::Right,
            true,
        )));
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "remote-id-2".into(),
            SyncDestination::Right
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
            "inbox".into(),
            Envelope {
                id: "local-id-2".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            SyncDestination::Left,
            SyncDestination::Right,
            true,
        )));
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "local-id-3".into(),
            SyncDestination::Left
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
            "inbox".into(),
            Envelope {
                id: "remote-id-3".into(),
                flags: "seen".into(),
                ..Envelope::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )));
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "local-id-4".into(),
            SyncDestination::Left
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
            "inbox".into(),
            Envelope {
                id: "remote-id-4".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
            true,
        )));
        assert!(patch.contains(&EmailSyncHunk::Delete(
            "inbox".into(),
            "local-id-5".into(),
            SyncDestination::Left
        )));
        assert!(patch.contains(&EmailSyncHunk::CopyThenCache(
            "inbox".into(),
            Envelope {
                id: "remote-id-5".into(),
                flags: "seen".into(),
                date: "2022-01-01T00:00:00-00:00".parse().unwrap(),
                ..Envelope::default()
            },
            SyncDestination::Right,
            SyncDestination::Left,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![
                EmailSyncHunk::Uncache("inbox".into(), "remote-id".into(), SyncDestination::Right),
                EmailSyncHunk::CopyThenCache(
                    "inbox".into(),
                    Envelope {
                        id: "local-id".into(),
                        flags: "seen".into(),
                        ..Envelope::default()
                    },
                    SyncDestination::Left,
                    SyncDestination::Right,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::GetThenCache(
                "inbox".into(),
                "local-id".into(),
                SyncDestination::Left,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::Uncache(
                "inbox".into(),
                "local-cache-id".into(),
                SyncDestination::Left
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![
                EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    SyncDestination::Left
                ),
                EmailSyncHunk::CopyThenCache(
                    "inbox".into(),
                    Envelope {
                        id: "remote-id".into(),
                        flags: "seen".into(),
                        ..Envelope::default()
                    },
                    SyncDestination::Right,
                    SyncDestination::Left,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    SyncDestination::Left
                )],
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    SyncDestination::Right
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    SyncDestination::Left,
                )],
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    SyncDestination::Right,
                )],
                vec![EmailSyncHunk::Delete(
                    "inbox".into(),
                    "remote-id".into(),
                    SyncDestination::Right
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::CopyThenCache(
                "inbox".into(),
                Envelope {
                    id: "local-id".into(),
                    flags: "seen".into(),
                    ..Envelope::default()
                },
                SyncDestination::Left,
                SyncDestination::Right,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::CopyThenCache(
                    "inbox".into(),
                    Envelope {
                        id: "local-id".into(),
                        flags: "flagged".into(),
                        ..Envelope::default()
                    },
                    SyncDestination::Left,
                    SyncDestination::Right,
                    false,
                )],
                vec![EmailSyncHunk::UpdateCachedFlags(
                    "inbox".into(),
                    Envelope {
                        id: "local-cache-id".into(),
                        flags: Flags::from_iter([Flag::Flagged]),
                        ..Envelope::default()
                    },
                    SyncDestination::Left,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([vec![EmailSyncHunk::GetThenCache(
                "inbox".into(),
                "remote-id".into(),
                SyncDestination::Right,
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
            super::build("inbox", local_cache, local, remote_cache, remote),
            EmailSyncPatch::from_iter([
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "local-cache-id".into(),
                    SyncDestination::Left,
                )],
                vec![EmailSyncHunk::Delete(
                    "inbox".into(),
                    "local-id".into(),
                    SyncDestination::Left
                )],
                vec![EmailSyncHunk::Uncache(
                    "inbox".into(),
                    "remote-cache-id".into(),
                    SyncDestination::Right,
                )],
            ])
        );
    }
}
