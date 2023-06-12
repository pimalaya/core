use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use log::{error, info, warn};
use rayon::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    env, fmt,
    fs::OpenOptions,
    io, result,
};
use thiserror::Error;

use crate::{
    account, envelope,
    folder::{
        self,
        sync::{FolderName, FoldersName},
    },
    AccountConfig, Backend, BackendBuilder, EnvelopeSyncPatch, EnvelopeSyncPatchManager,
    FolderSyncPatch, FolderSyncPatchManager, MaildirBackendBuilder, MaildirConfig,
};

use super::maildir;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot synchronize account {0}: synchronization not enabled")]
    SyncAccountNotEnabledError(String),
    #[error("cannot synchronize account {1}: cannot open lock file")]
    SyncAccountOpenLockFileError(#[source] io::Error, String),
    #[error("cannot synchronize account {1}: cannot lock process")]
    SyncAccountLockFileError(#[source] FileLockError, String),
    #[error("cannot synchronize account {1}: cannot unlock process")]
    SyncAccountUnlockFileError(#[source] FileLockError, String),

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    SyncFoldersError(#[from] folder::sync::Error),
    #[error(transparent)]
    SyncEnvelopesError(#[from] envelope::sync::Error),
    #[error(transparent)]
    BackendError(#[from] super::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] maildir::Error),
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Destination {
    Local,
    Remote,
}

impl fmt::Display for Destination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Remote => write!(f, "remote"),
        }
    }
}

pub type Id = String;
pub type Source = Destination;
pub type Target = Destination;
pub type RefreshSourceCache = bool;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendSyncProgressEvent {
    BuildFoldersDiffPatch,
    GetLocalCachedFolders,
    GetLocalFolders,
    GetRemoteCachedFolders,
    GetRemoteFolders,
    SynchronizeFolders(HashMap<folder::sync::FolderName, FolderSyncPatch>),
    SynchronizeFolder(folder::sync::FolderSyncHunk),

    BuildEnvelopesDiffPatches(folder::sync::FoldersName),
    EnvelopesDiffPatchBuilt(folder::sync::FolderName, EnvelopeSyncPatch),
    GetLocalCachedEnvelopes,
    GetLocalEnvelopes,
    GetRemoteCachedEnvelopes,
    GetRemoteEnvelopes,
    ProcessEnvelopePatches(HashMap<folder::sync::FolderName, EnvelopeSyncPatch>),
    ProcessEnvelopeHunk(envelope::sync::EnvelopeSyncHunk),
    ProcessEnvelopeCachePatch(Vec<envelope::sync::EnvelopeSyncCacheHunk>),

    ExpungeFolders(FoldersName),
    FolderExpunged(FolderName),
}

impl fmt::Display for BackendSyncProgressEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuildFoldersDiffPatch => write!(f, "Building folders diff patch"),
            Self::GetLocalCachedFolders => write!(f, "Getting local cached folders"),
            Self::GetLocalFolders => write!(f, "Getting local folders"),
            Self::GetRemoteCachedFolders => write!(f, "Getting remote cached folders"),
            Self::GetRemoteFolders => write!(f, "Getting remote folders"),
            Self::SynchronizeFolders(patches) => {
                let x = patches.values().fold(0, |sum, patch| sum + patch.len());
                let y = patches.len();
                write!(f, "Processing {x} patches of {y} folders")
            }
            Self::SynchronizeFolder(hunk) => write!(f, "{hunk}"),
            Self::BuildEnvelopesDiffPatches(folders) => {
                let n = folders.len();
                write!(f, "Building envelopes diff patch for {n} folders")
            }
            Self::EnvelopesDiffPatchBuilt(folder, patch) => {
                let n = patch.iter().fold(0, |sum, patch| sum + patch.len());
                write!(f, "Built {n} envelopes diff patch for folder {folder}")
            }
            Self::GetLocalCachedEnvelopes => write!(f, "Getting local cached envelopes"),
            Self::GetLocalEnvelopes => write!(f, "Getting local envelopes"),
            Self::GetRemoteCachedEnvelopes => write!(f, "Getting remote cached envelopes"),
            Self::GetRemoteEnvelopes => write!(f, "Getting remote envelopes"),
            Self::ProcessEnvelopePatches(_patches) => {
                write!(f, "Processing envelope patches")
            }
            Self::ProcessEnvelopeHunk(hunk) => write!(f, "{hunk}"),
            Self::ProcessEnvelopeCachePatch(_patch) => write!(f, "Processing envelope cache patch"),
            Self::ExpungeFolders(folders) => write!(f, "Expunging {} folders", folders.len()),
            Self::FolderExpunged(folder) => write!(f, "Folder {folder} successfully expunged"),
        }
    }
}

#[derive(Debug, Default)]
pub struct BackendSyncReport {
    pub folders: folder::sync::FoldersName,
    pub folders_patch: Vec<(folder::sync::FolderSyncHunk, Option<folder::sync::Error>)>,
    pub folders_cache_patch: (
        Vec<folder::sync::FolderSyncCacheHunk>,
        Option<folder::sync::Error>,
    ),
    pub envelopes_patch: Vec<(
        envelope::sync::EnvelopeSyncHunk,
        Option<envelope::sync::Error>,
    )>,
    pub envelopes_cache_patch: (
        Vec<envelope::sync::EnvelopeSyncCacheHunk>,
        Option<envelope::sync::Error>,
    ),
}

pub struct BackendSyncProgress<'a>(
    Box<dyn Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a>,
);

impl Default for BackendSyncProgress<'_> {
    fn default() -> Self {
        Self::new(|_evt| Ok(()))
    }
}

impl<'a> BackendSyncProgress<'a> {
    pub fn new(f: impl Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a) -> Self {
        Self(Box::new(f))
    }

    pub fn emit(&self, evt: BackendSyncProgressEvent) {
        if let Err(err) = (self.0)(evt.clone()) {
            warn!("error while emitting backend sync event {evt:?}, skipping it");
            error!("error while emitting backend sync event: {err:?}");
        }
    }
}

pub struct BackendSyncBuilder<'a> {
    account_config: AccountConfig,
    remote_builder: BackendBuilder,
    on_progress: BackendSyncProgress<'a>,
    folders_strategy: folder::sync::FolderSyncStrategy,
    dry_run: bool,
}

impl<'a> BackendSyncBuilder<'a> {
    pub fn new(account_config: AccountConfig, backend_builder: BackendBuilder) -> Result<Self> {
        let folders_strategy = account_config.sync_folders_strategy.clone();
        Ok(Self {
            account_config,
            remote_builder: backend_builder
                .with_cache_disabled(true)
                .with_default_credentials()?,
            on_progress: Default::default(),
            dry_run: Default::default(),
            folders_strategy,
        })
    }

    pub fn with_on_progress(
        mut self,
        f: impl Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a,
    ) -> Self {
        self.on_progress = BackendSyncProgress::new(f);
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_folders_strategy(mut self, strategy: folder::sync::FolderSyncStrategy) -> Self {
        self.folders_strategy = strategy;
        self
    }

    pub fn with_some_folders_strategy(
        mut self,
        strategy: Option<folder::sync::FolderSyncStrategy>,
    ) -> Self {
        if let Some(strategy) = strategy {
            self.folders_strategy = strategy;
        }
        self
    }

    pub fn sync(&self) -> Result<BackendSyncReport> {
        let account = &self.account_config.name;
        if !self.account_config.sync {
            return Err(Error::SyncAccountNotEnabledError(account.clone()));
        }

        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(env::temp_dir().join(format!("himalaya-sync-{}.lock", account)))
            .map_err(|err| Error::SyncAccountOpenLockFileError(err, account.clone()))?;
        lock_file
            .try_lock(FileLockMode::Exclusive)
            .map_err(|err| Error::SyncAccountLockFileError(err, account.clone()))?;

        info!("starting synchronization");
        let sync_dir = self.account_config.sync_dir()?;

        // init SQLite cache

        let conn = &mut self.account_config.sync_db_builder()?;

        folder::sync::Cache::init(conn)?;
        envelope::sync::Cache::init(conn)?;

        // init local Maildir

        let local_builder = MaildirBackendBuilder::new(
            self.account_config.clone(),
            MaildirConfig {
                root_dir: sync_dir.clone(),
            },
        );

        // apply folder aliases to the strategy
        let folders_strategy = match &self.folders_strategy {
            folder::sync::FolderSyncStrategy::All => folder::sync::FolderSyncStrategy::All,
            folder::sync::FolderSyncStrategy::Include(folders) => {
                folder::sync::FolderSyncStrategy::Include(
                    folders
                        .iter()
                        .map(|folder| Ok(self.account_config.folder_alias(folder)?))
                        .collect::<Result<_>>()?,
                )
            }
            folder::sync::FolderSyncStrategy::Exclude(folders) => {
                folder::sync::FolderSyncStrategy::Exclude(
                    folders
                        .iter()
                        .map(|folder| Ok(self.account_config.folder_alias(folder)?))
                        .collect::<Result<_>>()?,
                )
            }
        };

        self.on_progress
            .emit(BackendSyncProgressEvent::BuildFoldersDiffPatch);

        let folder_sync_patch_manager = FolderSyncPatchManager::new(
            &self.account_config,
            &local_builder,
            &self.remote_builder,
            &folders_strategy,
            &self.on_progress,
            self.dry_run,
        );
        let folder_sync_patch = folder_sync_patch_manager.build_patch()?;
        let folder_sync_report = folder_sync_patch_manager.apply_patch(folder_sync_patch)?;
        let folders = folder_sync_report.folders.clone();

        let envelope_patch_manager = EnvelopeSyncPatchManager::new(
            &self.account_config,
            &local_builder,
            &self.remote_builder,
            &self.on_progress,
            self.dry_run,
        );

        self.on_progress
            .emit(BackendSyncProgressEvent::BuildEnvelopesDiffPatches(
                folders.clone(),
            ));

        let envelopes_patches = HashMap::from_iter(
            folders
                .par_iter()
                .map(|folder| {
                    let patch = envelope_patch_manager.build_patch(folder)?;
                    Ok((folder.clone(), patch))
                })
                .collect::<Result<Vec<_>>>()?,
        );

        let envelopes_patch = envelopes_patches
            .values()
            .cloned()
            .flatten()
            .collect::<HashSet<_>>();

        self.on_progress
            .emit(BackendSyncProgressEvent::ProcessEnvelopePatches(
                envelopes_patches,
            ));

        let envelopes_sync_report = envelope_patch_manager.apply_patch(conn, envelopes_patch)?;

        self.on_progress
            .emit(BackendSyncProgressEvent::ExpungeFolders(folders.clone()));

        folders.par_iter().try_for_each(|folder| {
            local_builder.build()?.expunge_folder(folder)?;
            self.remote_builder.build()?.expunge_folder(folder)?;
            self.on_progress
                .emit(BackendSyncProgressEvent::FolderExpunged(folder.clone()));
            Result::Ok(())
        })?;

        lock_file
            .unlock()
            .map_err(|err| Error::SyncAccountUnlockFileError(err, account.clone()))?;

        Ok(BackendSyncReport {
            folders,
            folders_patch: folder_sync_report.patch,
            folders_cache_patch: folder_sync_report.cache_patch,
            envelopes_patch: envelopes_sync_report.patch,
            envelopes_cache_patch: envelopes_sync_report.cache_patch,
        })
    }
}
