use log::{debug, info, trace, warn};
use rayon::prelude::*;
use std::{collections::HashSet, fmt};

use crate::{
    AccountConfig, Backend, BackendBuilder, BackendSyncProgressEvent, MaildirBackendBuilder,
};

use super::{Cache, Error, Result};

pub type FoldersName = HashSet<FolderName>;
pub type FolderName = String;
pub type Patch = Vec<Hunk>;
pub type Target = HunkKind;
pub type TargetRestricted = HunkKindRestricted;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Strategy {
    #[default]
    All,
    Include(HashSet<String>),
    Exclude(HashSet<String>),
}

impl Strategy {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HunkKind {
    LocalCache,
    Local,
    RemoteCache,
    Remote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HunkKindRestricted {
    Local,
    Remote,
}

impl fmt::Display for HunkKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalCache => write!(f, "local cache"),
            Self::Local => write!(f, "local backend"),
            Self::RemoteCache => write!(f, "remote cache"),
            Self::Remote => write!(f, "remote backend"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Hunk {
    CreateFolder(FolderName, Target),
    DeleteFolder(FolderName, Target),
}

impl fmt::Display for Hunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CreateFolder(name, target) => write!(f, "Adding folder {name} to {target}"),
            Self::DeleteFolder(name, target) => write!(f, "Removing folder {name} from {target}"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CacheHunk {
    CreateFolder(FolderName, TargetRestricted),
    DeleteFolder(FolderName, TargetRestricted),
}

#[derive(Debug, Default)]
pub struct SyncReport {
    pub folders: FoldersName,
    pub patch: Vec<(Hunk, Option<Error>)>,
    pub cache_patch: (Vec<CacheHunk>, Option<Error>),
}

pub struct SyncBuilder<'a> {
    account_config: AccountConfig,
    on_progress: Box<dyn Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a>,
    strategy: Strategy,
    dry_run: bool,
}

impl<'a> SyncBuilder<'a> {
    pub fn new(account_config: AccountConfig) -> Self {
        let strategy = account_config.sync_folders_strategy.clone();
        Self {
            account_config,
            on_progress: Box::new(|_| Ok(())),
            strategy,
            dry_run: false,
        }
    }

    pub fn on_progress<F>(mut self, f: F) -> Self
    where
        F: Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a,
    {
        self.on_progress = Box::new(f);
        self
    }

    fn try_progress(&self, evt: BackendSyncProgressEvent) {
        let progress = &self.on_progress;
        if let Err(err) = progress(evt.clone()) {
            warn!("error while emitting event {evt:?}: {err}");
        }
    }

    pub fn strategy(mut self, strategy: Strategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn sync(
        &self,
        conn: &mut rusqlite::Connection,
        local_builder: &MaildirBackendBuilder,
        remote_builder: &BackendBuilder,
    ) -> Result<SyncReport> {
        let account = &self.account_config.name;
        info!("starting folders synchronization of account {account}");

        self.try_progress(BackendSyncProgressEvent::GetLocalCachedFolders);

        let local_folders_cached: FoldersName = HashSet::from_iter(
            Cache::list_local_folders(conn, account, &self.strategy)?
                .iter()
                .cloned(),
        );

        trace!("local folders cached: {:#?}", local_folders_cached);

        self.try_progress(BackendSyncProgressEvent::GetLocalFolders);

        let local_folders: FoldersName = HashSet::from_iter(
            local_builder
                .build()?
                .list_folders()
                .map_err(Box::new)?
                .iter()
                // TODO: instead of fetching all the folders then
                // filtering them here, it could be better to filter
                // them at the source directly, which implies to add a
                // new backend fn called `search_folders` and to set
                // up a common search API across backends.
                .filter_map(|folder| match &self.strategy {
                    Strategy::All => Some(folder.name.clone()),
                    Strategy::Include(folders) => {
                        if folders.contains(&folder.name) {
                            Some(folder.name.clone())
                        } else {
                            None
                        }
                    }
                    Strategy::Exclude(folders) => {
                        if folders.contains(&folder.name) {
                            None
                        } else {
                            Some(folder.name.clone())
                        }
                    }
                }),
        );

        trace!("local folders: {:#?}", local_folders);

        self.try_progress(BackendSyncProgressEvent::GetRemoteCachedFolders);

        let remote_folders_cached: FoldersName = HashSet::from_iter(
            Cache::list_remote_folders(conn, account, &self.strategy)?
                .iter()
                .cloned(),
        );

        trace!("remote folders cached: {:#?}", remote_folders_cached);

        self.try_progress(BackendSyncProgressEvent::GetRemoteFolders);

        let remote_folders: FoldersName = HashSet::from_iter(
            remote_builder
                .build()
                .map_err(Box::new)?
                .list_folders()
                .map_err(Box::new)?
                .iter()
                // TODO: instead of fetching all the folders then
                // filtering them here, it could be better to filter
                // them at the source directly, which implies to add a
                // new backend fn called `search_folders` and to set
                // up a common search API across backends.
                .filter_map(|folder| match &self.strategy {
                    Strategy::All => Some(folder.name.clone()),
                    Strategy::Include(folders) => {
                        if folders.contains(&folder.name) {
                            Some(folder.name.clone())
                        } else {
                            None
                        }
                    }
                    Strategy::Exclude(folders) => {
                        if folders.contains(&folder.name) {
                            None
                        } else {
                            Some(folder.name.clone())
                        }
                    }
                }),
        );

        trace!("remote folders: {:#?}", remote_folders);

        self.try_progress(BackendSyncProgressEvent::BuildFoldersPatch);

        let (patch, folders) = build_patch(
            local_folders_cached,
            local_folders,
            remote_folders_cached,
            remote_folders,
        );

        self.try_progress(BackendSyncProgressEvent::ProcessFoldersPatch(patch.len()));

        debug!("folders patch: {:#?}", patch);

        let mut report = SyncReport::default();

        if self.dry_run {
            info!("dry run enabled, skipping folders patch");
            report.patch = patch.into_iter().map(|patch| (patch, None)).collect();
        } else {
            let process_hunk = |hunk: &Hunk| {
                Result::Ok(match hunk {
                    Hunk::CreateFolder(folder, HunkKind::LocalCache) => {
                        vec![CacheHunk::CreateFolder(
                            folder.clone(),
                            TargetRestricted::Local,
                        )]
                    }
                    Hunk::CreateFolder(ref folder, HunkKind::Local) => {
                        local_builder
                            .build()?
                            .add_folder(folder)
                            .map_err(Box::new)?;
                        vec![]
                    }
                    Hunk::CreateFolder(ref folder, HunkKind::RemoteCache) => {
                        vec![CacheHunk::CreateFolder(
                            folder.clone(),
                            TargetRestricted::Remote,
                        )]
                    }
                    Hunk::CreateFolder(ref folder, HunkKind::Remote) => {
                        remote_builder
                            .build()
                            .map_err(Box::new)?
                            .add_folder(&folder)
                            .map_err(Box::new)?;
                        vec![]
                    }
                    Hunk::DeleteFolder(ref folder, HunkKind::LocalCache) => {
                        vec![CacheHunk::DeleteFolder(
                            folder.clone(),
                            TargetRestricted::Local,
                        )]
                    }
                    Hunk::DeleteFolder(ref folder, HunkKind::Local) => {
                        local_builder
                            .build()?
                            .delete_folder(folder)
                            .map_err(Box::new)?;
                        vec![]
                    }
                    Hunk::DeleteFolder(ref folder, HunkKind::RemoteCache) => {
                        vec![CacheHunk::DeleteFolder(
                            folder.clone(),
                            TargetRestricted::Remote,
                        )]
                    }
                    Hunk::DeleteFolder(ref folder, HunkKind::Remote) => {
                        remote_builder
                            .build()
                            .map_err(Box::new)?
                            .delete_folder(&folder)
                            .map_err(Box::new)?;
                        vec![]
                    }
                })
            };

            report = patch
                .par_iter()
                .fold(SyncReport::default, |mut report, hunk| {
                    let hunk_str = hunk.to_string();

                    trace!("processing hunk: {hunk:#?}");
                    debug!("{hunk_str}");

                    self.try_progress(BackendSyncProgressEvent::ProcessFolderHunk(hunk_str));

                    match process_hunk(hunk) {
                        Ok(cache_hunks) => {
                            report.patch.push((hunk.clone(), None));
                            report.cache_patch.0.extend(cache_hunks);
                        }
                        Err(err) => {
                            warn!("error while processing hunk {hunk:?}, skipping it: {err:?}");
                            report.patch.push((hunk.clone(), Some(err)));
                        }
                    };

                    report
                })
                .reduce(SyncReport::default, |mut r1, r2| {
                    r1.patch.extend(r2.patch);
                    r1.cache_patch.0.extend(r2.cache_patch.0);
                    r1
                });

            let mut process_cache_patch = || {
                let tx = conn.transaction()?;
                for hunk in &report.cache_patch.0 {
                    match hunk {
                        CacheHunk::CreateFolder(folder, TargetRestricted::Local) => {
                            Cache::insert_local_folder(&tx, account, folder)?;
                        }
                        CacheHunk::CreateFolder(folder, TargetRestricted::Remote) => {
                            Cache::insert_remote_folder(&tx, account, folder)?;
                        }
                        CacheHunk::DeleteFolder(folder, TargetRestricted::Local) => {
                            Cache::delete_local_folder(&tx, account, folder)?;
                        }
                        CacheHunk::DeleteFolder(folder, TargetRestricted::Remote) => {
                            Cache::delete_remote_folder(&tx, account, folder)?;
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

        report.folders = folders
            .into_iter()
            .map(|folder| {
                urlencoding::decode(&folder)
                    .map(|folder| folder.to_string())
                    .unwrap_or_else(|_| folder)
            })
            .collect();

        trace!("sync report: {:#?}", report);

        Ok(report)
    }
}

pub fn build_patch(
    local_cache: FoldersName,
    local: FoldersName,
    remote_cache: FoldersName,
    remote: FoldersName,
) -> (Patch, FoldersName) {
    let mut patch: Patch = vec![];
    let mut folders: FoldersName = HashSet::new();

    // Gathers all existing folders name.
    folders.extend(local_cache.clone());
    folders.extend(local.clone());
    folders.extend(remote_cache.clone());
    folders.extend(remote.clone());

    // Given the matrice local_cache × local × remote_cache × remote,
    // checks every 2⁴ = 16 possibilities:
    for folder in &folders {
        let local_cache = local_cache.get(folder);
        let local = local.get(folder);
        let remote_cache = remote_cache.get(folder);
        let remote = remote.get(folder);

        match (local_cache, local, remote_cache, remote) {
            // 0000
            (None, None, None, None) => (),

            // 0001
            (None, None, None, Some(_)) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::Local),
                Hunk::CreateFolder(folder.clone(), HunkKind::RemoteCache),
            ]),

            // 0010
            (None, None, Some(_), None) => {
                patch.push(Hunk::DeleteFolder(folder.clone(), HunkKind::RemoteCache))
            }

            // 0011
            (None, None, Some(_), Some(_)) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::Local),
            ]),

            // 0100
            //
            (None, Some(_), None, None) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::RemoteCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::Remote),
            ]),

            // 0101
            (None, Some(_), None, Some(_)) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::RemoteCache),
            ]),

            // 0110
            (None, Some(_), Some(_), None) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::Remote),
            ]),

            // 0111
            (None, Some(_), Some(_), Some(_)) => {
                patch.push(Hunk::CreateFolder(folder.clone(), HunkKind::LocalCache))
            }

            // 1000
            (Some(_), None, None, None) => {
                patch.push(Hunk::DeleteFolder(folder.clone(), HunkKind::LocalCache))
            }

            // 1001
            (Some(_), None, None, Some(_)) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::Local),
                Hunk::CreateFolder(folder.clone(), HunkKind::RemoteCache),
            ]),

            // 1010
            (Some(_), None, Some(_), None) => patch.extend([
                Hunk::DeleteFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::DeleteFolder(folder.clone(), HunkKind::RemoteCache),
            ]),

            // 1011
            (Some(_), None, Some(_), Some(_)) => patch.extend([
                Hunk::DeleteFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::DeleteFolder(folder.clone(), HunkKind::RemoteCache),
                Hunk::DeleteFolder(folder.clone(), HunkKind::Remote),
            ]),

            // 1100
            (Some(_), Some(_), None, None) => patch.extend([
                Hunk::CreateFolder(folder.clone(), HunkKind::RemoteCache),
                Hunk::CreateFolder(folder.clone(), HunkKind::Remote),
            ]),

            // 1101
            (Some(_), Some(_), None, Some(_)) => {
                patch.push(Hunk::CreateFolder(folder.clone(), HunkKind::RemoteCache))
            }

            // 1110
            (Some(_), Some(_), Some(_), None) => patch.extend([
                Hunk::DeleteFolder(folder.clone(), HunkKind::LocalCache),
                Hunk::DeleteFolder(folder.clone(), HunkKind::Local),
                Hunk::DeleteFolder(folder.clone(), HunkKind::RemoteCache),
            ]),

            // 1111
            (Some(_), Some(_), Some(_), Some(_)) => (),
        }
    }

    (patch, folders)
}

#[cfg(test)]
mod folders_sync {
    use super::{FoldersName, Hunk, HunkKind, Patch};

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
            (vec![] as Patch, FoldersName::default()),
        );

        // 0001
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::Local),
                    Hunk::CreateFolder("folder".into(), HunkKind::RemoteCache),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 0010
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            (
                vec![Hunk::DeleteFolder("folder".into(), HunkKind::RemoteCache)],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 0011
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::Local),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 0100
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::RemoteCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::Remote),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 0101
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::RemoteCache),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 0110
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::Remote),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 0111
        assert_eq!(
            super::build_patch(
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![Hunk::CreateFolder("folder".into(), HunkKind::LocalCache)],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1000
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::default(),
            ),
            (
                vec![Hunk::DeleteFolder("folder".into(), HunkKind::LocalCache)],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1001
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::Local),
                    Hunk::CreateFolder("folder".into(), HunkKind::RemoteCache),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1010
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            (
                vec![
                    Hunk::DeleteFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::DeleteFolder("folder".into(), HunkKind::RemoteCache),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1011
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![
                    Hunk::DeleteFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::DeleteFolder("folder".into(), HunkKind::RemoteCache),
                    Hunk::DeleteFolder("folder".into(), HunkKind::Remote),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1100
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::default(),
            ),
            (
                vec![
                    Hunk::CreateFolder("folder".into(), HunkKind::RemoteCache),
                    Hunk::CreateFolder("folder".into(), HunkKind::Remote),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1101
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
                FoldersName::from_iter(["folder".into()]),
            ),
            (
                vec![Hunk::CreateFolder("folder".into(), HunkKind::RemoteCache)],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1110
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::default(),
            ),
            (
                vec![
                    Hunk::DeleteFolder("folder".into(), HunkKind::LocalCache),
                    Hunk::DeleteFolder("folder".into(), HunkKind::Local),
                    Hunk::DeleteFolder("folder".into(), HunkKind::RemoteCache),
                ],
                FoldersName::from_iter(["folder".into()]),
            ),
        );

        // 1111
        assert_eq!(
            super::build_patch(
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
                FoldersName::from_iter(["folder".into()]),
            ),
            (vec![] as Patch, FoldersName::from_iter(["folder".into()])),
        );
    }
}
