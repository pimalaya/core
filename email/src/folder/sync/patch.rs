//! Module dedicated to email folders synchronization patch.
//!
//! The core structure of the module is the [`FolderSyncPatch`], which
//! represents a list of changes (hunks).
//!
//! You also have access to a [`FolderSyncPatchManager`] which helps
//! you to build and to apply a folder patch.

use futures::{stream, StreamExt};
use log::{debug, info, trace, warn};
use std::collections::{HashMap, HashSet};

use crate::{
    account::{
        sync::{AccountSyncProgress, AccountSyncProgressEvent, Destination},
        AccountConfig,
    },
    backend::{Backend, BackendBuilderV2, BackendContextBuilder, MaildirBackendBuilder},
    folder::{add::AddFolder, delete::DeleteFolder, list::ListFolders},
    Result,
};

use super::*;

/// A folder synchronization patch is just a list of folder
/// synchronization hunks (changes).
pub type FolderSyncPatch = Vec<FolderSyncHunk>;

/// A folder synchronization patches associates a folder with its own
/// patch.
pub type FolderSyncPatches = HashMap<FolderName, FolderSyncPatch>;

/// A folder synchronization cache patch is just a list of folder
/// synchronization cache hunks (changes).
pub type FolderSyncCachePatch = Vec<FolderSyncCacheHunk>;

/// The folder synchronization patch manager.
///
/// This structure helps you to build a patch and to apply it.
pub struct FolderSyncPatchManager<'a, B: BackendContextBuilder> {
    account_config: &'a AccountConfig,
    local_builder: MaildirBackendBuilder,
    remote_builder_v2: BackendBuilderV2<B>,
    strategy: &'a FolderSyncStrategy,
    on_progress: AccountSyncProgress,
    dry_run: bool,
}

impl<'a, B: BackendContextBuilder + 'static> FolderSyncPatchManager<'a, B> {
    /// Creates a new folder synchronization patch manager.
    pub fn new(
        account_config: &'a AccountConfig,
        local_builder: MaildirBackendBuilder,
        remote_builder_v2: BackendBuilderV2<B>,
        strategy: &'a FolderSyncStrategy,
        on_progress: AccountSyncProgress,
        dry_run: bool,
    ) -> Self {
        Self {
            account_config,
            local_builder,
            remote_builder_v2,
            strategy,
            on_progress,
            dry_run,
        }
    }

    /// Builds the folder synchronization patches.
    pub async fn build_patches(&self) -> Result<FolderSyncPatches> {
        let account = &self.account_config.name;
        let conn = &mut self.account_config.sync_db_builder()?;
        info!("starting folders synchronization of account {account}");

        self.on_progress
            .emit(AccountSyncProgressEvent::GetLocalCachedFolders);

        let local_folders_cached: FoldersName = HashSet::from_iter(
            FolderSyncCache::list_local_folders(conn, account, &self.strategy)?
                .iter()
                .cloned(),
        );

        trace!("local folders cached: {:#?}", local_folders_cached);

        self.on_progress
            .emit(AccountSyncProgressEvent::GetLocalFolders);

        let local_folders: FoldersName = HashSet::from_iter(
            self.local_builder
                .build()?
                .list_folders()
                .await?
                .iter()
                // TODO: instead of fetching all the folders then
                // filtering them here, it could be better to filter
                // them at the source directly, which implies to add a
                // new backend fn called `search_folders` and to set
                // up a common search API across backends.
                .filter_map(|folder| match &self.strategy {
                    FolderSyncStrategy::All => Some(folder.name.clone()),
                    FolderSyncStrategy::Include(folders) => {
                        if folders.contains(&folder.name) {
                            Some(folder.name.clone())
                        } else {
                            None
                        }
                    }
                    FolderSyncStrategy::Exclude(folders) => {
                        if folders.contains(&folder.name) {
                            None
                        } else {
                            Some(folder.name.clone())
                        }
                    }
                }),
        );

        trace!("local folders: {:#?}", local_folders);

        self.on_progress
            .emit(AccountSyncProgressEvent::GetRemoteCachedFolders);

        let remote_folders_cached: FoldersName = HashSet::from_iter(
            FolderSyncCache::list_remote_folders(conn, account, &self.strategy)?
                .iter()
                .cloned(),
        );

        trace!("remote folders cached: {:#?}", remote_folders_cached);

        self.on_progress
            .emit(AccountSyncProgressEvent::GetRemoteFolders);

        let remote_folders: FoldersName = HashSet::from_iter(
            self.remote_builder_v2
                .clone()
                .build()
                .await?
                .list_folders()
                .await?
                .iter()
                // TODO: instead of fetching all the folders then
                // filtering them here, it could be better to filter
                // them at the source directly, which implies to add a
                // new backend fn called `search_folders` and to set
                // up a common search API across backends.
                .filter_map(|folder| match &self.strategy {
                    FolderSyncStrategy::All => Some(folder.name.clone()),
                    FolderSyncStrategy::Include(folders) => {
                        if folders.contains(&folder.name) {
                            Some(folder.name.clone())
                        } else {
                            None
                        }
                    }
                    FolderSyncStrategy::Exclude(folders) => {
                        if folders.contains(&folder.name) {
                            None
                        } else {
                            Some(folder.name.clone())
                        }
                    }
                }),
        );

        trace!("remote folders: {:#?}", remote_folders);

        let patches = build_patch(
            local_folders_cached,
            local_folders,
            remote_folders_cached,
            remote_folders,
        );

        self.on_progress
            .emit(AccountSyncProgressEvent::ApplyFolderPatches(
                patches.clone(),
            ));

        debug!("folders patches: {:#?}", patches);

        Ok(patches)
    }

    async fn process_hunk(
        local_builder: MaildirBackendBuilder,
        remote_builder_v2: BackendBuilderV2<B>,
        hunk: &FolderSyncHunk,
    ) -> Result<FolderSyncCachePatch> {
        let cache_hunks = match &hunk {
            FolderSyncHunk::Cache(folder, Destination::Local) => {
                vec![FolderSyncCacheHunk::Insert(
                    folder.clone(),
                    Destination::Local,
                )]
            }
            FolderSyncHunk::Create(ref folder, Destination::Local) => {
                local_builder.build()?.add_folder(folder).await?;
                vec![]
            }
            FolderSyncHunk::Cache(ref folder, Destination::Remote) => {
                vec![FolderSyncCacheHunk::Insert(
                    folder.clone(),
                    Destination::Remote,
                )]
            }
            FolderSyncHunk::Create(ref folder, Destination::Remote) => {
                remote_builder_v2.build().await?.add_folder(&folder).await?;
                vec![]
            }
            FolderSyncHunk::Uncache(ref folder, Destination::Local) => {
                vec![FolderSyncCacheHunk::Delete(
                    folder.clone(),
                    Destination::Local,
                )]
            }
            FolderSyncHunk::Delete(ref folder, Destination::Local) => {
                local_builder.build()?.delete_folder(folder).await?;
                vec![]
            }
            FolderSyncHunk::Uncache(ref folder, Destination::Remote) => {
                vec![FolderSyncCacheHunk::Delete(
                    folder.clone(),
                    Destination::Remote,
                )]
            }
            FolderSyncHunk::Delete(ref folder, Destination::Remote) => {
                remote_builder_v2
                    .build()
                    .await?
                    .delete_folder(&folder)
                    .await?;
                vec![]
            }
        };

        Ok(cache_hunks)
    }

    /// Applies all the folder synchronization patches built from
    /// `build_patches()`.
    ///
    /// Returns a folder synchronization report.
    pub async fn apply_patches(&self, patches: FolderSyncPatches) -> Result<FolderSyncReport> {
        let account = &self.account_config.name;
        let conn = &mut self.account_config.sync_db_builder()?;
        let mut report = FolderSyncReport::default();

        let folders = patches
            .iter()
            .map(|(folder, _patch)| {
                urlencoding::decode(folder)
                    .map(|folder| folder.to_string())
                    .unwrap_or_else(|_| folder.clone())
            })
            .collect();

        if self.dry_run {
            info!("dry run enabled, skipping folders patch");
            report.patch = patches
                .iter()
                .flat_map(|(_folder, patch)| patch)
                .map(|patch| (patch.clone(), None))
                .collect();
        } else {
            report = stream::iter(patches.into_iter().flat_map(|(_folder, patch)| patch))
                .map(|hunk| {
                    let on_progress = self.on_progress.clone();
                    let local_builder = self.local_builder.clone();
                    let remote_builder_v2 = self.remote_builder_v2.clone();

                    tokio::spawn(async move {
                        debug!("processing folder hunk: {hunk:?}");

                        let mut report = FolderSyncReport::default();

                        on_progress.emit(AccountSyncProgressEvent::ApplyFolderHunk(hunk.clone()));

                        match Self::process_hunk(local_builder, remote_builder_v2, &hunk).await {
                            Ok(cache_hunks) => {
                                report.patch.push((hunk.clone(), None));
                                report.cache_patch.0.extend(cache_hunks);
                            }
                            Err(err) => {
                                warn!("error while processing folder hunk: {err}");
                                debug!("error while processing folder hunk: {err:?}");
                                report.patch.push((hunk.clone(), Some(err)));
                            }
                        };

                        Result::Ok(report)
                    })
                })
                .buffer_unordered(16)
                .filter_map(|report| async {
                    match report {
                        Ok(Ok(report)) => Some(report),
                        _ => None,
                    }
                })
                .fold(FolderSyncReport::default(), |mut r1, r2| async {
                    r1.patch.extend(r2.patch);
                    r1.cache_patch.0.extend(r2.cache_patch.0);
                    r1
                })
                .await;

            let mut process_cache_patch = || {
                let tx = conn.transaction()?;
                for hunk in &report.cache_patch.0 {
                    match hunk {
                        FolderSyncCacheHunk::Insert(folder, Destination::Local) => {
                            FolderSyncCache::insert_local_folder(&tx, account, folder)?;
                        }
                        FolderSyncCacheHunk::Insert(folder, Destination::Remote) => {
                            FolderSyncCache::insert_remote_folder(&tx, account, folder)?;
                        }
                        FolderSyncCacheHunk::Delete(folder, Destination::Local) => {
                            FolderSyncCache::delete_local_folder(&tx, account, folder)?;
                        }
                        FolderSyncCacheHunk::Delete(folder, Destination::Remote) => {
                            FolderSyncCache::delete_remote_folder(&tx, account, folder)?;
                        }
                    }
                }
                tx.commit()?;
                Result::Ok(())
            };

            if let Err(err) = process_cache_patch() {
                warn!("error while processing cache patch: {err}");
                report.cache_patch.1 = Some(err);
            }
        };

        report.folders = folders;

        trace!("sync report: {:#?}", report);

        Ok(report)
    }
}

/// Folder synchronization patch builder.
///
/// Contains the core algorithm of the folder synchronization. It has
/// been exported in a dedicated function so that it can be easily
/// tested.
pub fn build_patch(
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
                FolderSyncHunk::Cache(folder.clone(), Destination::Local),
                FolderSyncHunk::Create(folder.clone(), Destination::Local),
                FolderSyncHunk::Cache(folder.clone(), Destination::Remote),
            ],

            // 0010
            (None, None, Some(_), None) => {
                vec![FolderSyncHunk::Uncache(folder.clone(), Destination::Remote)]
            }

            // 0011
            (None, None, Some(_), Some(_)) => vec![
                FolderSyncHunk::Cache(folder.clone(), Destination::Local),
                FolderSyncHunk::Create(folder.clone(), Destination::Local),
            ],

            // 0100
            //
            (None, Some(_), None, None) => vec![
                FolderSyncHunk::Cache(folder.clone(), Destination::Local),
                FolderSyncHunk::Cache(folder.clone(), Destination::Remote),
                FolderSyncHunk::Create(folder.clone(), Destination::Remote),
            ],

            // 0101
            (None, Some(_), None, Some(_)) => vec![
                FolderSyncHunk::Cache(folder.clone(), Destination::Local),
                FolderSyncHunk::Cache(folder.clone(), Destination::Remote),
            ],

            // 0110
            (None, Some(_), Some(_), None) => vec![
                FolderSyncHunk::Cache(folder.clone(), Destination::Local),
                FolderSyncHunk::Create(folder.clone(), Destination::Remote),
            ],

            // 0111
            (None, Some(_), Some(_), Some(_)) => {
                vec![FolderSyncHunk::Cache(folder.clone(), Destination::Local)]
            }

            // 1000
            (Some(_), None, None, None) => {
                vec![FolderSyncHunk::Uncache(folder.clone(), Destination::Local)]
            }

            // 1001
            (Some(_), None, None, Some(_)) => vec![
                FolderSyncHunk::Create(folder.clone(), Destination::Local),
                FolderSyncHunk::Cache(folder.clone(), Destination::Remote),
            ],

            // 1010
            (Some(_), None, Some(_), None) => vec![
                FolderSyncHunk::Uncache(folder.clone(), Destination::Local),
                FolderSyncHunk::Uncache(folder.clone(), Destination::Remote),
            ],

            // 1011
            (Some(_), None, Some(_), Some(_)) => vec![
                FolderSyncHunk::Uncache(folder.clone(), Destination::Local),
                FolderSyncHunk::Uncache(folder.clone(), Destination::Remote),
                FolderSyncHunk::Delete(folder.clone(), Destination::Remote),
            ],

            // 1100
            (Some(_), Some(_), None, None) => vec![
                FolderSyncHunk::Cache(folder.clone(), Destination::Remote),
                FolderSyncHunk::Create(folder.clone(), Destination::Remote),
            ],

            // 1101
            (Some(_), Some(_), None, Some(_)) => {
                vec![FolderSyncHunk::Cache(folder.clone(), Destination::Remote)]
            }

            // 1110
            (Some(_), Some(_), Some(_), None) => vec![
                FolderSyncHunk::Uncache(folder.clone(), Destination::Local),
                FolderSyncHunk::Delete(folder.clone(), Destination::Local),
                FolderSyncHunk::Uncache(folder.clone(), Destination::Remote),
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

    use crate::account::sync::Destination;

    use super::{FolderSyncHunk, FoldersName};

    #[test]
    fn build_folder_patch() {
        // 0000
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::new()
        );

        // 0001
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), Destination::Local),
                    FolderSyncHunk::Create("folder".into(), Destination::Local),
                    FolderSyncHunk::Cache("folder".into(), Destination::Remote),
                ]
            )]),
        );

        // 0010
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([(
                "folder".into(),
                vec![FolderSyncHunk::Uncache(
                    "folder".into(),
                    Destination::Remote
                )],
            )]),
        );

        // 0011
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), Destination::Local),
                    FolderSyncHunk::Create("folder".into(), Destination::Local),
                ],
            ))]),
        );

        // 0100
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), Destination::Local),
                    FolderSyncHunk::Cache("folder".into(), Destination::Remote),
                    FolderSyncHunk::Create("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 0101
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), Destination::Local),
                    FolderSyncHunk::Cache("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 0110
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), Destination::Local),
                    FolderSyncHunk::Create("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 0111
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![FolderSyncHunk::Cache("folder".into(), Destination::Local)],
            ))]),
        );

        // 1000
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![FolderSyncHunk::Uncache("folder".into(), Destination::Local)],
            ))]),
        );

        // 1001
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Create("folder".into(), Destination::Local),
                    FolderSyncHunk::Cache("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 1010
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Uncache("folder".into(), Destination::Local),
                    FolderSyncHunk::Uncache("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 1011
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Uncache("folder".into(), Destination::Local),
                    FolderSyncHunk::Uncache("folder".into(), Destination::Remote),
                    FolderSyncHunk::Delete("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 1100
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Cache("folder".into(), Destination::Remote),
                    FolderSyncHunk::Create("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 1101
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![FolderSyncHunk::Cache("folder".into(), Destination::Remote)],
            ))]),
        );

        // 1110
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            HashMap::from_iter([((
                "folder".into(),
                vec![
                    FolderSyncHunk::Uncache("folder".into(), Destination::Local),
                    FolderSyncHunk::Delete("folder".into(), Destination::Local),
                    FolderSyncHunk::Uncache("folder".into(), Destination::Remote),
                ],
            ))]),
        );

        // 1111
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            HashMap::from_iter([("folder".into(), vec![])])
        );
    }
}
