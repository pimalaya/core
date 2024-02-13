//! Module dedicated to email folders synchronization patch.
//!
//! The core structure of the module is the [`FolderSyncPatch`], which
//! represents a list of changes (hunks).
//!
//! You also have access to a [`FolderSyncPatchManager`] which helps
//! you to build and to apply a folder patch.

use std::collections::{HashMap, HashSet};

use crate::sync::SyncDestination;

use super::hunk::{FolderName, FolderSyncHunk, FoldersName};

/// A folder synchronization patch is just a list of folder
/// synchronization hunks (changes).
pub type FolderSyncPatch = Vec<FolderSyncHunk>;

/// A folder synchronization patches associates a folder with its own
/// patch.
pub type FolderSyncPatches = HashMap<FolderName, FolderSyncPatch>;

/// Folder synchronization patch builder.
///
/// Contains the core algorithm of the folder synchronization. It has
/// been exported in a dedicated function so that it can be easily
/// tested.
pub fn build(
    local_cache: FoldersName,
    local: FoldersName,
    remote_cache: FoldersName,
    remote: FoldersName,
) -> HashMap<FolderName, FolderSyncPatch> {
    let mut folders = HashSet::new();

    // Gathers all existing folders name.
    folders.extend(local_cache.clone());
    folders.extend(local.clone());
    folders.extend(remote_cache.clone());
    folders.extend(remote.clone());

    // Given the matrix local_cache × local × remote_cache × remote,
    // checks every 2⁴ = 16 possibilities:
    let patches = folders.into_iter().map(|folder| {
        let local_cache = local_cache.get(&folder);
        let local = local.get(&folder);
        let remote_cache = remote_cache.get(&folder);
        let remote = remote.get(&folder);

        let patch = match (local_cache, local, remote_cache, remote) {
            // 0000
            (None, None, None, None) => vec![],

            // 0001
            (None, None, None, Some(_)) => vec![
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Create(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Right),
            ],

            // 0010
            (None, None, Some(_), None) => {
                vec![FolderSyncHunk::Uncache(
                    folder.clone(),
                    SyncDestination::Right,
                )]
            }

            // 0011
            (None, None, Some(_), Some(_)) => vec![
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Create(folder.clone(), SyncDestination::Left),
            ],

            // 0100
            (None, Some(_), None, None) => vec![
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Right),
                FolderSyncHunk::Create(folder.clone(), SyncDestination::Right),
            ],

            // 0101
            (None, Some(_), None, Some(_)) => vec![
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Right),
            ],

            // 0110
            (None, Some(_), Some(_), None) => vec![
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Create(folder.clone(), SyncDestination::Right),
            ],

            // 0111
            (None, Some(_), Some(_), Some(_)) => {
                vec![FolderSyncHunk::Cache(folder.clone(), SyncDestination::Left)]
            }

            // 1000
            (Some(_), None, None, None) => {
                vec![FolderSyncHunk::Uncache(
                    folder.clone(),
                    SyncDestination::Left,
                )]
            }

            // 1001
            (Some(_), None, None, Some(_)) => vec![
                FolderSyncHunk::Create(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Right),
            ],

            // 1010
            (Some(_), None, Some(_), None) => vec![
                FolderSyncHunk::Uncache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Uncache(folder.clone(), SyncDestination::Right),
            ],

            // 1011
            (Some(_), None, Some(_), Some(_)) => vec![
                FolderSyncHunk::Uncache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Uncache(folder.clone(), SyncDestination::Right),
                FolderSyncHunk::Delete(folder.clone(), SyncDestination::Right),
            ],

            // 1100
            (Some(_), Some(_), None, None) => vec![
                FolderSyncHunk::Cache(folder.clone(), SyncDestination::Right),
                FolderSyncHunk::Create(folder.clone(), SyncDestination::Right),
            ],

            // 1101
            (Some(_), Some(_), None, Some(_)) => {
                vec![FolderSyncHunk::Cache(
                    folder.clone(),
                    SyncDestination::Right,
                )]
            }

            // 1110
            (Some(_), Some(_), Some(_), None) => vec![
                FolderSyncHunk::Uncache(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Delete(folder.clone(), SyncDestination::Left),
                FolderSyncHunk::Uncache(folder.clone(), SyncDestination::Right),
            ],

            // 1111
            (Some(_), Some(_), Some(_), Some(_)) => vec![],
        };

        (folder, patch)
    });

    HashMap::from_iter(patches)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::sync::SyncDestination;

    use super::{FolderSyncHunk, FoldersName};

    #[test]
    fn build_folder_patch() {
        // 0000
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::new()
        );

        // 0001
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Create("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Right),
                ]
            )]),
        );

        // 0010
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![FolderSyncHunk::Uncache(
                    "folder".into(),
                    SyncDestination::Right
                )],
            )]),
        );

        // 0011
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Create("folder".into(), SyncDestination::Left),
                ],
            )]),
        );

        // 0100
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Right),
                    FolderSyncHunk::Create("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 0101
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 0110
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Create("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 0111
        assert_eq!(
            super::build(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![FolderSyncHunk::Cache(
                    "folder".into(),
                    SyncDestination::Left
                )],
            )]),
        );

        // 1000
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![FolderSyncHunk::Uncache(
                    "folder".into(),
                    SyncDestination::Left
                )],
            )]),
        );

        // 1001
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Create("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 1010
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Uncache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Uncache("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 1011
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Uncache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Uncache("folder".into(), SyncDestination::Right),
                    FolderSyncHunk::Delete("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 1100
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), SyncDestination::Right),
                    FolderSyncHunk::Create("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 1101
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![FolderSyncHunk::Cache(
                    "folder".into(),
                    SyncDestination::Right
                )],
            )]),
        );

        // 1110
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Uncache("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Delete("folder".into(), SyncDestination::Left),
                    FolderSyncHunk::Uncache("folder".into(), SyncDestination::Right),
                ],
            )]),
        );

        // 1111
        assert_eq!(
            super::build(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([("folder".into(), vec![])])
        );
    }
}
