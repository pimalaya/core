//! Module dedicated to email envelope flags synchronization.
//!
//! This module contains a single function [sync] that synchronizes
//! multiple email envelope flags sources.

pub mod config;

use std::collections::HashSet;

use super::{Flag, Flags};

/// The email envelope flag synchronizer.
///
/// Folds multiple source of flags into one synchronized flags.
pub fn sync(
    local_cache: Option<&Flags>,
    local: Option<&Flags>,
    remote_cache: Option<&Flags>,
    remote: Option<&Flags>,
) -> Flags {
    let mut synchronized_flags: HashSet<Flag> = HashSet::default();

    let mut all_flags: HashSet<Flag> = HashSet::default();
    all_flags.extend(local_cache.map(|e| e.0.clone()).unwrap_or_default());
    all_flags.extend(local.map(|e| e.0.clone()).unwrap_or_default());
    all_flags.extend(remote_cache.map(|e| e.0.clone()).unwrap_or_default());
    all_flags.extend(remote.map(|e| e.0.clone()).unwrap_or_default());

    for flag in all_flags {
        match (
            local_cache.and_then(|e| e.get(&flag)),
            local.and_then(|e| e.get(&flag)),
            remote_cache.and_then(|e| e.get(&flag)),
            remote.and_then(|e| e.get(&flag)),
        ) {
            // The flag exists nowhere, which cannot happen since the
            // flags hashset is built from envelopes flags.
            (None, None, None, None) => (),

            // The flag only exists in remote side, which means a new
            // flag has been added.
            (None, None, None, Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag only exists in remote cache, which means an
            // outdated flag needs to be removed.
            (None, None, Some(_), None) => {
                synchronized_flags.remove(&flag);
            }

            // The flag exists in remote side but not in local side,
            // which means there is a conflict. Since we cannot
            // determine which side (local removed or remote added) is
            // the most up-to-date, it is safer to consider the remote
            // added side up-to-date (or local removed in case of
            // [`Flag::Deleted`]) in order not to lose data.
            //
            // TODO: make this behaviour customizable.
            (None, None, Some(_), Some(_)) if flag == Flag::Deleted => {
                synchronized_flags.remove(&flag);
            }
            (None, None, Some(_), Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag only exists in local side, which means a new
            // flag has been added.
            (None, Some(_), None, None) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag exists in local and remote sides, which means
            // a new (same) flag has been added both sides at the same
            // time.
            (None, Some(_), None, Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag exists in local side and remote cache side,
            // which means a new (same) flag has been added local side
            // but removed remote side. Since we cannot determine
            // which side (local added or remote removed) is the most
            // up-to-date, it is safer to consider the local added
            // side up-to-date (or remote removed in case of
            // [`Flag::Deleted`]) in order not to lose data.
            //
            // TODO: make this behaviour customizable.
            (None, Some(_), Some(_), None) if flag == Flag::Deleted => {
                synchronized_flags.remove(&flag);
            }
            (None, Some(_), Some(_), None) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag exists everywhere except in local cache, which
            // means the local cache misses a flag.
            (None, Some(_), Some(_), Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag only exists in local cache side, which means
            // the local cache has an outdated flag.
            (Some(_), None, None, None) => {
                synchronized_flags.remove(&flag);
            }

            // The flag exists in local cache side and remote side,
            // which means a new (same) flag has been removed local
            // cache side but added remote side. Since we cannot
            // determine which side (local removed or remote added) is
            // the most up-to-date, it is safer to consider the remote
            // added side up-to-date (or local removed in case of
            // [`Flag::Deleted`]) in order not to lose data.
            //
            // TODO: make this behaviour customizable.
            (Some(_), None, None, Some(_)) if flag == Flag::Deleted => {
                synchronized_flags.remove(&flag);
            }
            (Some(_), None, None, Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag exists in both caches, which means a old flag
            // needs to be removed everywhere.
            (Some(_), None, Some(_), None) => {
                synchronized_flags.remove(&flag);
            }

            // The flag exists everywhere except in local side, which
            // means a flag has been removed local side and needs to
            // be removed everywhere else.
            (Some(_), None, Some(_), Some(_)) => {
                synchronized_flags.remove(&flag);
            }

            // The flag exists in the local sides but not in remote
            // sides, which means there is a conflict. Since we cannot
            // determine which side is the most up-to-date, it is
            // safer to consider the local side side up-to-date (or
            // remote side in case of [`Flag::Deleted`]) in order not
            // to lose data.
            //
            // TODO: make this behaviour customizable.
            (Some(_), Some(_), None, None) if flag == Flag::Deleted => {
                synchronized_flags.remove(&flag);
            }
            (Some(_), Some(_), None, None) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag exists everywhere except in remote cache side,
            // which means the remote cache misses a flag.
            (Some(_), Some(_), None, Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }

            // The flag exists everywhere except in remote side, which
            // means a flag has been removed remote side and needs to
            // be removed everywhere else.
            (Some(_), Some(_), Some(_), None) => {
                synchronized_flags.remove(&flag);
            }

            // The flag exists everywhere, which means the flag needs
            // to be added.
            (Some(_), Some(_), Some(_), Some(_)) => {
                synchronized_flags.insert(flag.clone());
            }
        }
    }

    Flags::from_iter(synchronized_flags)
}

#[cfg(test)]
mod tests {
    use crate::flag::{Flag, Flags};

    #[test]
    fn sync() {
        assert_eq!(super::sync(None, None, None, None), Flags::default());

        assert_eq!(
            super::sync(None, None, None, Some(&Flags::from_iter([Flag::Seen]))),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::default()),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
            ),
            Flags::default()
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::default()),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::default()),
                Some(&Flags::default()),
            ),
            Flags::default()
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
            ),
            Flags::default(),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::default(),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::default()),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
                Some(&Flags::from_iter([Flag::Seen])),
            ),
            Flags::from_iter([Flag::Seen]),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::from_iter([Flag::Seen])),
                Some(&Flags::default()),
            ),
            Flags::default(),
        );

        assert_eq!(
            super::sync(
                Some(&Flags::from_iter([Flag::Seen, Flag::Flagged])),
                Some(&Flags::from_iter([Flag::Seen, Flag::Flagged])),
                Some(&Flags::from_iter([Flag::Seen, Flag::Flagged])),
                Some(&Flags::from_iter([Flag::Seen, Flag::Flagged])),
            ),
            Flags::from_iter([Flag::Seen, Flag::Flagged]),
        );
    }
}
