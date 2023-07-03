//! Module dedicated to account synchronization.
//!
//! The core concept of this module is the [`AccountSyncBuilder`],
//! which allows you to synchronize folders and emails for a given
//! account using a Maildir backend.

use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, error, info, warn};
use std::{
    collections::{HashMap, HashSet},
    env, fmt,
    fs::OpenOptions,
    io,
};
use thiserror::Error;

use crate::{
    account::AccountConfig,
    backend::{Backend, BackendBuilder, MaildirBackendBuilder, MaildirConfig},
    email::sync::{
        EmailSyncCache, EmailSyncCacheHunk, EmailSyncCachePatch, EmailSyncHunk, EmailSyncPatch,
        EmailSyncPatchManager,
    },
    folder::sync::{
        FolderName, FolderSyncCache, FolderSyncCacheHunk, FolderSyncHunk, FolderSyncPatchManager,
        FolderSyncPatches, FolderSyncStrategy, FoldersName,
    },
    Result,
};

/// Errors related to account synchronization.
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
}

/// The synchronization destination.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum Destination {
    /// An item needs to be synchronized to the local Maildir.
    Local,

    /// An item needs to be synchronized remotely.
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

/// Alias for the source destination.
pub type Source = Destination;

/// Alias for the target destination.
pub type Target = Destination;

/// The backend synchronization progress event.
///
/// Represents all the events that can be triggered during the backend
/// synchronization process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccountSyncProgressEvent {
    BuildFolderPatch,
    GetLocalCachedFolders,
    GetLocalFolders,
    GetRemoteCachedFolders,
    GetRemoteFolders,
    ApplyFolderPatches(FolderSyncPatches),
    ApplyFolderHunk(FolderSyncHunk),

    BuildEnvelopePatch(FoldersName),
    EnvelopePatchBuilt(FolderName, EmailSyncPatch),
    GetLocalCachedEnvelopes,
    GetLocalEnvelopes,
    GetRemoteCachedEnvelopes,
    GetRemoteEnvelopes,
    ApplyEnvelopePatches(HashMap<FolderName, EmailSyncPatch>),
    ApplyEnvelopeHunk(EmailSyncHunk),
    ApplyEnvelopeCachePatch(EmailSyncCachePatch),

    ExpungeFolders(FoldersName),
    FolderExpunged(FolderName),
}

impl fmt::Display for AccountSyncProgressEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BuildFolderPatch => write!(f, "Building folders diff patch"),
            Self::GetLocalCachedFolders => write!(f, "Getting local cached folders"),
            Self::GetLocalFolders => write!(f, "Getting local folders"),
            Self::GetRemoteCachedFolders => write!(f, "Getting remote cached folders"),
            Self::GetRemoteFolders => write!(f, "Getting remote folders"),
            Self::ApplyFolderPatches(patches) => {
                let x = patches.values().fold(0, |sum, patch| sum + patch.len());
                let y = patches.len();
                write!(f, "Processing {x} patches of {y} folders")
            }
            Self::ApplyFolderHunk(hunk) => write!(f, "{hunk}"),
            Self::BuildEnvelopePatch(folders) => {
                let n = folders.len();
                write!(f, "Building envelopes diff patch for {n} folders")
            }
            Self::EnvelopePatchBuilt(folder, patch) => {
                let n = patch.iter().fold(0, |sum, patch| sum + patch.len());
                write!(f, "Built {n} envelopes diff patch for folder {folder}")
            }
            Self::GetLocalCachedEnvelopes => write!(f, "Getting local cached envelopes"),
            Self::GetLocalEnvelopes => write!(f, "Getting local envelopes"),
            Self::GetRemoteCachedEnvelopes => write!(f, "Getting remote cached envelopes"),
            Self::GetRemoteEnvelopes => write!(f, "Getting remote envelopes"),
            Self::ApplyEnvelopePatches(_patches) => {
                write!(f, "Processing envelope patches")
            }
            Self::ApplyEnvelopeHunk(hunk) => write!(f, "{hunk}"),
            Self::ApplyEnvelopeCachePatch(_patch) => {
                write!(f, "Processing envelope cache patch")
            }
            Self::ExpungeFolders(folders) => write!(f, "Expunging {} folders", folders.len()),
            Self::FolderExpunged(folder) => write!(f, "Folder {folder} successfully expunged"),
        }
    }
}

/// The account synchronization report.
///
/// Gathers folder and email synchronization reports in one unique
/// report.
#[derive(Debug, Default)]
pub struct AccountSyncReport {
    pub folders: FoldersName,
    pub folders_patch: Vec<(FolderSyncHunk, Option<crate::Error>)>,
    pub folders_cache_patch: (Vec<FolderSyncCacheHunk>, Option<crate::Error>),
    pub emails_patch: Vec<(EmailSyncHunk, Option<crate::Error>)>,
    pub emails_cache_patch: (Vec<EmailSyncCacheHunk>, Option<crate::Error>),
}

/// The account synchronization progress callback.
pub struct AccountSyncProgress<'a>(
    Box<dyn Fn(AccountSyncProgressEvent) -> Result<()> + Sync + Send + 'a>,
);

impl Default for AccountSyncProgress<'_> {
    fn default() -> Self {
        Self::new(|_evt| Ok(()))
    }
}

impl<'a> AccountSyncProgress<'a> {
    pub fn new(f: impl Fn(AccountSyncProgressEvent) -> Result<()> + Sync + Send + 'a) -> Self {
        Self(Box::new(f))
    }

    pub fn emit(&self, evt: AccountSyncProgressEvent) {
        debug!("emitting sync progress event {evt:?}");
        if let Err(err) = (self.0)(evt.clone()) {
            warn!("error while emitting backend sync event {evt:?}, skipping it");
            error!("error while emitting backend sync event: {err:?}");
        }
    }
}

/// The account synchronization builder.
///
/// This is not really a builder since there is no `build()` function,
/// but it follows the builder pattern. When all the options are set
/// up, `sync()` synchronizes the current account locally, using the
/// given remote builder.
pub struct AccountSyncBuilder<'a> {
    account_config: AccountConfig,
    remote_builder: BackendBuilder,
    on_progress: AccountSyncProgress<'a>,
    folders_strategy: FolderSyncStrategy,
    dry_run: bool,
}

impl<'a> AccountSyncBuilder<'a> {
    /// Creates a new account synchronization builder.
    pub async fn new(
        account_config: AccountConfig,
        remote_builder: BackendBuilder,
    ) -> Result<AccountSyncBuilder<'a>> {
        let folders_strategy = account_config.sync_folders_strategy.clone();
        Ok(Self {
            account_config,
            remote_builder: remote_builder
                .with_cache_disabled(true)
                .with_default_credentials()
                .await?,
            on_progress: Default::default(),
            dry_run: Default::default(),
            folders_strategy,
        })
    }

    /// Sets the progress callback following the builder pattern.
    pub fn with_on_progress(
        mut self,
        f: impl Fn(AccountSyncProgressEvent) -> Result<()> + Sync + Send + 'a,
    ) -> Self {
        self.on_progress = AccountSyncProgress::new(f);
        self
    }

    /// Sets the dry run flag following the builder pattern.
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Sets the sync folders strategy following the builder pattern.
    pub fn with_folders_strategy(mut self, strategy: FolderSyncStrategy) -> Self {
        self.folders_strategy = strategy;
        self
    }

    /// Sets the sync folders strategy following the builder pattern.
    ///
    /// Like `with_folders_strategy()`, but takes an optional strategy
    /// instead.
    pub fn with_some_folders_strategy(mut self, strategy: Option<FolderSyncStrategy>) -> Self {
        if let Some(strategy) = strategy {
            self.folders_strategy = strategy;
        }
        self
    }

    /// Synchronizes the current account locally, using a Maildir
    /// backend.
    ///
    /// Acts like a `build()` function in a regular builder pattern,
    /// except that the synchronizer builder is not consumed.
    pub async fn sync(&self) -> Result<AccountSyncReport> {
        let account = &self.account_config.name;
        info!("starting synchronization of account {account}");

        if !self.account_config.sync {
            warn!("sync feature not enabled for account {account}, aborting");
            return Ok(Err(Error::SyncAccountNotEnabledError(account.clone()))?);
        }

        let lock_file_path = env::temp_dir().join(format!("himalaya-sync-{}.lock", account));
        debug!("locking sync file {lock_file_path:?}");

        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(lock_file_path)
            .map_err(|err| Error::SyncAccountOpenLockFileError(err, account.clone()))?;
        lock_file
            .try_lock(FileLockMode::Exclusive)
            .map_err(|err| Error::SyncAccountLockFileError(err, account.clone()))?;

        let sync_dir = self.account_config.sync_dir()?;

        debug!("initializing folder and envelope cache");
        let conn = &mut self.account_config.sync_db_builder()?;
        FolderSyncCache::init(conn)?;
        EmailSyncCache::init(conn)?;

        let local_builder = MaildirBackendBuilder::new(
            self.account_config.clone(),
            MaildirConfig {
                root_dir: sync_dir.clone(),
            },
        );

        debug!("applying folder aliases to the folder sync strategy");
        let folders_strategy = match &self.folders_strategy {
            FolderSyncStrategy::All => FolderSyncStrategy::All,
            FolderSyncStrategy::Include(folders) => FolderSyncStrategy::Include(
                folders
                    .iter()
                    .map(|folder| Ok(self.account_config.folder_alias(folder)?))
                    .collect::<Result<_>>()?,
            ),
            FolderSyncStrategy::Exclude(folders) => FolderSyncStrategy::Exclude(
                folders
                    .iter()
                    .map(|folder| Ok(self.account_config.folder_alias(folder)?))
                    .collect::<Result<_>>()?,
            ),
        };

        self.on_progress
            .emit(AccountSyncProgressEvent::BuildFolderPatch);

        let folder_sync_patch_manager = FolderSyncPatchManager::new(
            &self.account_config,
            &local_builder,
            &self.remote_builder,
            &folders_strategy,
            &self.on_progress,
            self.dry_run,
        );

        debug!("building folder sync patch");
        let folder_sync_patch = folder_sync_patch_manager.build_patches().await?;
        debug!("{folder_sync_patch:#?}");

        debug!("applying folder sync patch");
        let folder_sync_report = folder_sync_patch_manager
            .apply_patches(folder_sync_patch)
            .await?;
        debug!("{folder_sync_report:#?}");

        let folders = folder_sync_report.folders.clone();

        self.on_progress
            .emit(AccountSyncProgressEvent::BuildEnvelopePatch(
                folders.clone(),
            ));

        let envelope_sync_patch_manager = EmailSyncPatchManager::new(
            &self.account_config,
            &local_builder,
            &self.remote_builder,
            &self.on_progress,
            self.dry_run,
        );

        debug!("building envelope sync patch");
        let envelope_sync_patches =
            FuturesUnordered::from_iter(folders.iter().map(|folder| async {
                let patch = envelope_sync_patch_manager
                    .build_patch(folder.clone())
                    .await?;
                Ok((folder.clone(), patch))
            }))
            .collect::<Vec<Result<_>>>()
            .await;
        let envelope_sync_patches = envelope_sync_patches
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let envelope_sync_patches = HashMap::from_iter(envelope_sync_patches);
        debug!("{envelope_sync_patches:#?}");

        let envelope_sync_patch = envelope_sync_patches
            .values()
            .cloned()
            .flatten()
            .collect::<HashSet<_>>();

        self.on_progress
            .emit(AccountSyncProgressEvent::ApplyEnvelopePatches(
                envelope_sync_patches,
            ));

        debug!("applying envelope sync patch");
        let envelope_sync_report = envelope_sync_patch_manager
            .apply_patch(conn, envelope_sync_patch)
            .await?;
        debug!("{envelope_sync_report:#?}");

        self.on_progress
            .emit(AccountSyncProgressEvent::ExpungeFolders(folders.clone()));

        debug!("expunging folders");
        FuturesUnordered::from_iter(folders.iter().map(|folder| async {
            local_builder.build()?.expunge_folder(folder).await?;
            self.remote_builder
                .build()
                .await?
                .expunge_folder(folder)
                .await?;
            self.on_progress
                .emit(AccountSyncProgressEvent::FolderExpunged(folder.clone()));
            Ok(())
        }))
        .collect::<Vec<Result<()>>>()
        .await;

        debug!("unlocking sync file");
        lock_file
            .unlock()
            .map_err(|err| Error::SyncAccountUnlockFileError(err, account.clone()))?;

        debug!("building final sync report");
        let sync_report = AccountSyncReport {
            folders,
            folders_patch: folder_sync_report.patch,
            folders_cache_patch: folder_sync_report.cache_patch,
            emails_patch: envelope_sync_report.patch,
            emails_cache_patch: envelope_sync_report.cache_patch,
        };
        debug!("{sync_report:#?}");

        Ok(sync_report)
    }
}
