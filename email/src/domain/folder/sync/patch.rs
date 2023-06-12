use log::{debug, error, info, trace, warn};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::{
    backend::sync::Destination, AccountConfig, Backend, BackendBuilder, BackendSyncProgress,
    BackendSyncProgressEvent, MaildirBackendBuilder,
};

use super::*;

pub type FolderSyncCachePatch = Vec<FolderSyncCacheHunk>;

pub struct FolderSyncPatchManager<'a> {
    account_config: &'a AccountConfig,
    local_builder: &'a MaildirBackendBuilder,
    remote_builder: &'a BackendBuilder,
    strategy: &'a FolderSyncStrategy,
    on_progress: &'a BackendSyncProgress<'a>,
    dry_run: bool,
}

impl<'a> FolderSyncPatchManager<'a> {
    pub fn new(
        account_config: &'a AccountConfig,
        local_builder: &'a MaildirBackendBuilder,
        remote_builder: &'a BackendBuilder,
        strategy: &'a FolderSyncStrategy,
        on_progress: &'a BackendSyncProgress<'a>,
        dry_run: bool,
    ) -> Self {
        Self {
            account_config,
            local_builder,
            remote_builder,
            strategy,
            on_progress,
            dry_run,
        }
    }

    pub fn build_patch(&self) -> Result<FolderSyncPatches> {
        let account = &self.account_config.name;
        let conn = &mut self.account_config.sync_db_builder()?;
        info!("starting folders synchronization of account {account}");

        self.on_progress
            .emit(BackendSyncProgressEvent::GetLocalCachedFolders);

        let local_folders_cached: FoldersName = HashSet::from_iter(
            FolderSyncCache::list_local_folders(conn, account, &self.strategy)?
                .iter()
                .cloned(),
        );

        trace!("local folders cached: {:#?}", local_folders_cached);

        self.on_progress
            .emit(BackendSyncProgressEvent::GetLocalFolders);

        let local_folders: FoldersName = HashSet::from_iter(
            self.local_builder
                .build()?
                .list_folders()?
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
            .emit(BackendSyncProgressEvent::GetRemoteCachedFolders);

        let remote_folders_cached: FoldersName = HashSet::from_iter(
            FolderSyncCache::list_remote_folders(conn, account, &self.strategy)?
                .iter()
                .cloned(),
        );

        trace!("remote folders cached: {:#?}", remote_folders_cached);

        self.on_progress
            .emit(BackendSyncProgressEvent::GetRemoteFolders);

        let remote_folders: FoldersName = HashSet::from_iter(
            self.remote_builder
                .build()?
                .list_folders()?
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
            .emit(BackendSyncProgressEvent::ApplyFolderPatches(
                patches.clone(),
            ));

        debug!("folders patches: {:#?}", patches);

        Ok(patches)
    }

    pub fn apply_patch(&self, patches: FolderSyncPatches) -> Result<FolderSyncReport> {
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
            report = patches
                .into_par_iter()
                .flat_map(|(_folder, patch)| patch)
                .fold(FolderSyncReport::default, |mut report, ref hunk| {
                    debug!("processing folder hunk: {hunk:?}");

                    self.on_progress
                        .emit(BackendSyncProgressEvent::ApplyFolderHunk(hunk.clone()));

                    let process_hunk = |hunk: &FolderSyncHunk| {
                        Result::Ok(match hunk {
                            FolderSyncHunk::Cache(folder, Destination::Local) => {
                                vec![FolderSyncCacheHunk::Insert(
                                    folder.clone(),
                                    Destination::Local,
                                )]
                            }
                            FolderSyncHunk::Create(ref folder, Destination::Local) => {
                                self.local_builder.build()?.add_folder(folder)?;
                                vec![]
                            }
                            FolderSyncHunk::Cache(ref folder, Destination::Remote) => {
                                vec![FolderSyncCacheHunk::Insert(
                                    folder.clone(),
                                    Destination::Remote,
                                )]
                            }
                            FolderSyncHunk::Create(ref folder, Destination::Remote) => {
                                self.remote_builder.build()?.add_folder(&folder)?;
                                vec![]
                            }
                            FolderSyncHunk::Uncache(ref folder, Destination::Local) => {
                                vec![FolderSyncCacheHunk::Delete(
                                    folder.clone(),
                                    Destination::Local,
                                )]
                            }
                            FolderSyncHunk::Delete(ref folder, Destination::Local) => {
                                self.local_builder.build()?.delete_folder(folder)?;
                                vec![]
                            }
                            FolderSyncHunk::Uncache(ref folder, Destination::Remote) => {
                                vec![FolderSyncCacheHunk::Delete(
                                    folder.clone(),
                                    Destination::Remote,
                                )]
                            }
                            FolderSyncHunk::Delete(ref folder, Destination::Remote) => {
                                self.remote_builder.build()?.delete_folder(&folder)?;
                                vec![]
                            }
                        })
                    };

                    match process_hunk(hunk) {
                        Ok(cache_hunks) => {
                            report.patch.push((hunk.clone(), None));
                            report.cache_patch.0.extend(cache_hunks);
                        }
                        Err(err) => {
                            warn!("error while processing hunk {hunk:?}, skipping it: {err:?}");
                            error!("{err}");
                            report.patch.push((hunk.clone(), Some(err)));
                        }
                    };

                    report
                })
                .reduce(FolderSyncReport::default, |mut r1, r2| {
                    r1.patch.extend(r2.patch);
                    r1.cache_patch.0.extend(r2.cache_patch.0);
                    r1
                });

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

    // Given the matrice local_cache × local × remote_cache × remote,
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

    use crate::backend::sync::Destination;

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
