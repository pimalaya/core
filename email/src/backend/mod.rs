mod config;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;

use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use log::{error, info, warn};
use rayon::prelude::*;
use std::{
    any::Any,
    collections::{HashMap, HashSet},
    env, fmt,
    fs::OpenOptions,
    io, result,
};
use thiserror::Error;

pub use self::config::BackendConfig;
#[cfg(feature = "imap-backend")]
pub use self::imap::*;
pub use self::maildir::*;
#[cfg(feature = "notmuch-backend")]
pub use self::notmuch::*;
use crate::{
    account, email, envelope, folder, AccountConfig, Emails, Envelope, Envelopes, Flag, Flags,
    Folders,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build backend with an empty config")]
    BuildBackendError,
    #[error("cannot synchronize account {0}: synchronization not enabled")]
    SyncAccountNotEnabledError(String),
    #[error("cannot synchronize account {1}: cannot open lock file")]
    SyncAccountOpenLockFileError(#[source] io::Error, String),
    #[error("cannot synchronize account {1}: cannot lock process")]
    SyncAccountLockFileError(#[source] FileLockError, String),
    #[error("cannot synchronize account {1}: cannot unlock process")]
    SyncAccountUnlockFileError(#[source] FileLockError, String),

    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    SyncFoldersError(#[from] folder::sync::Error),
    #[error(transparent)]
    SyncEnvelopesError(#[from] envelope::sync::Error),
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),

    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapBackendError(#[from] imap::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapBackendConfigError(#[from] imap::config::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] maildir::Error),
    #[cfg(feature = "notmuch-backend")]
    #[error(transparent)]
    NotmuchBackendError(#[from] notmuch::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub trait Backend {
    fn name(&self) -> String;

    fn add_folder(&mut self, folder: &str) -> Result<()>;
    fn list_folders(&mut self) -> Result<Folders>;
    fn expunge_folder(&mut self, folder: &str) -> Result<()>;
    fn purge_folder(&mut self, folder: &str) -> Result<()>;
    fn delete_folder(&mut self, folder: &str) -> Result<()>;

    fn get_envelope(&mut self, folder: &str, id: &str) -> Result<Envelope>;
    fn list_envelopes(&mut self, folder: &str, page_size: usize, page: usize) -> Result<Envelopes>;
    fn search_envelopes(
        &mut self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    fn add_email(&mut self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;
    fn preview_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Emails>;
    fn get_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<Emails>;
    fn copy_emails(&mut self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn move_emails(&mut self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn delete_emails(&mut self, folder: &str, ids: Vec<&str>) -> Result<()>;

    fn add_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn set_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn remove_flags(&mut self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;

    fn mark_emails_as_deleted(&mut self, folder: &str, ids: Vec<&str>) -> Result<()> {
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Deleted]))
    }

    fn close(&mut self) -> Result<()> {
        Ok(())
    }

    fn try_clone(&self) -> Result<Box<dyn Backend>>;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendSyncProgressEvent {
    BuildFoldersDiffPatch,
    GetLocalCachedFolders,
    GetLocalFolders,
    GetRemoteCachedFolders,
    GetRemoteFolders,
    SynchronizeFolders(HashMap<folder::sync::FolderName, folder::sync::Patch>),
    SynchronizeFolder(folder::sync::Hunk),

    BuildEnvelopesDiffPatches(folder::sync::FoldersName),
    EnvelopesDiffPatchBuilt(folder::sync::FolderName, envelope::sync::Patch),
    GetLocalCachedEnvelopes,
    GetLocalEnvelopes,
    GetRemoteCachedEnvelopes,
    GetRemoteEnvelopes,
    ProcessEnvelopePatches(HashMap<folder::sync::FolderName, envelope::sync::Patch>),
    ProcessEnvelopeHunk(envelope::sync::BackendHunk),
    ProcessEnvelopeCachePatch(Vec<envelope::sync::CacheHunk>),

    ExpungeFolders(folder::sync::FoldersName),
    FolderExpunged(folder::sync::FolderName),
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
    pub folders_patch: Vec<(folder::sync::Hunk, Option<folder::sync::Error>)>,
    pub folders_cache_patch: (Vec<folder::sync::CacheHunk>, Option<folder::sync::Error>),
    pub envelopes_patch: Vec<(envelope::sync::BackendHunk, Option<envelope::sync::Error>)>,
    pub envelopes_cache_patch: (
        Vec<envelope::sync::CacheHunk>,
        Option<envelope::sync::Error>,
    ),
}

pub struct BackendSyncBuilder<'a> {
    account_config: AccountConfig,
    remote_builder: BackendBuilder,
    on_progress: Box<dyn Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a>,
    folders_strategy: folder::sync::Strategy,
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
            on_progress: Box::new(|_| Ok(())),
            folders_strategy,
            dry_run: false,
        })
    }

    pub fn on_progress<F>(mut self, f: F) -> Self
    where
        F: Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a,
    {
        self.on_progress = Box::new(f);
        self
    }

    pub fn folders_strategy(mut self, strategy: folder::sync::Strategy) -> Self {
        self.folders_strategy = strategy;
        self
    }

    pub fn some_folders_strategy(mut self, strategy: Option<folder::sync::Strategy>) -> Self {
        if let Some(strategy) = strategy {
            self.folders_strategy = strategy;
        }
        self
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    fn try_progress(&self, evt: BackendSyncProgressEvent) {
        let progress = &self.on_progress;

        if let Err(err) = progress(evt.clone()) {
            warn!("error while emitting event {evt:?}, skipping it");
            error!("error while emitting event: {err:?}");
        }
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
        let progress = &self.on_progress;
        let sync_dir = self.account_config.sync_dir()?;

        // init SQLite cache

        let db_builder = || Ok(rusqlite::Connection::open(sync_dir.join(".sync.sqlite"))?);
        let conn = &mut db_builder()?;

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
            folder::sync::Strategy::All => folder::sync::Strategy::All,
            folder::sync::Strategy::Include(folders) => folder::sync::Strategy::Include(
                folders
                    .iter()
                    .map(|folder| Ok(self.account_config.folder_alias(folder)?))
                    .collect::<Result<_>>()?,
            ),
            folder::sync::Strategy::Exclude(folders) => folder::sync::Strategy::Exclude(
                folders
                    .iter()
                    .map(|folder| Ok(self.account_config.folder_alias(folder)?))
                    .collect::<Result<_>>()?,
            ),
        };

        self.try_progress(BackendSyncProgressEvent::BuildFoldersDiffPatch);

        let folders_sync_report = folder::SyncBuilder::new(self.account_config.clone())
            .on_progress(|data| Ok(progress(data).map_err(Box::new)?))
            .strategy(folders_strategy)
            .dry_run(self.dry_run)
            .sync(conn, &local_builder, &self.remote_builder)?;
        let folders = folders_sync_report.folders.clone();

        let envelopes = envelope::SyncBuilder::new(self.account_config.clone())
            .on_progress(|data| Ok(progress(data).map_err(Box::new)?))
            .dry_run(self.dry_run);

        self.try_progress(BackendSyncProgressEvent::BuildEnvelopesDiffPatches(
            folders.clone(),
        ));

        let envelopes_patches = HashMap::from_iter(
            folders
                .par_iter()
                .map(|folder| {
                    Ok((
                        folder.clone(),
                        envelopes.build_patch(
                            folder,
                            &db_builder,
                            &local_builder,
                            &self.remote_builder,
                        )?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        );

        let envelopes_patch = envelopes_patches
            .values()
            .cloned()
            .flatten()
            .collect::<HashSet<_>>();

        self.try_progress(BackendSyncProgressEvent::ProcessEnvelopePatches(
            envelopes_patches,
        ));

        let envelopes_sync_report = envelopes.sync(
            Vec::from_iter(envelopes_patch),
            conn,
            &local_builder,
            &self.remote_builder,
        )?;

        self.try_progress(BackendSyncProgressEvent::ExpungeFolders(folders.clone()));

        folders.par_iter().try_for_each(|folder| {
            local_builder.build()?.expunge_folder(folder)?;
            self.remote_builder.build()?.expunge_folder(folder)?;
            self.try_progress(BackendSyncProgressEvent::FolderExpunged(folder.clone()));
            Result::Ok(())
        })?;

        lock_file
            .unlock()
            .map_err(|err| Error::SyncAccountUnlockFileError(err, account.clone()))?;

        Ok(BackendSyncReport {
            folders,
            folders_patch: folders_sync_report.patch,
            folders_cache_patch: folders_sync_report.cache_patch,
            envelopes_patch: envelopes_sync_report.patch,
            envelopes_cache_patch: envelopes_sync_report.cache_patch,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendBuilder {
    account_config: AccountConfig,
    default_credentials: Option<String>,
    disable_cache: bool,
}

impl BackendBuilder {
    pub fn new(account_config: AccountConfig) -> Self {
        Self {
            account_config,
            ..Default::default()
        }
    }

    pub fn with_cache_disabled(mut self, disable_cache: bool) -> Self {
        self.disable_cache = disable_cache;
        self
    }

    pub fn with_default_credentials(mut self) -> Result<Self> {
        self.default_credentials = match &self.account_config.backend {
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Some(imap_config.build_credentials()?)
            }
            _ => None,
        };
        Ok(self)
    }

    pub fn disable_cache(&mut self, disable_cache: bool) {
        self.disable_cache = disable_cache;
    }

    pub fn build(&self) -> Result<Box<dyn Backend>> {
        match &self.account_config.backend {
            BackendConfig::None => Err(Error::BuildBackendError),
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Ok(Box::new(ImapBackend::new(
                    self.account_config.clone(),
                    imap_config.clone(),
                    self.default_credentials.clone(),
                )?))
            }
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(_) => {
                let root_dir = self.account_config.sync_dir()?;
                Ok(Box::new(MaildirBackend::new(
                    self.account_config.clone(),
                    MaildirConfig { root_dir },
                )?))
            }
            BackendConfig::Maildir(mdir_config) => Ok(Box::new(MaildirBackend::new(
                self.account_config.clone(),
                mdir_config.clone(),
            )?)),
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(notmuch_config) => Ok(Box::new(NotmuchBackend::new(
                self.account_config.clone(),
                notmuch_config.clone(),
            )?)),
        }
    }

    pub fn into_build(self) -> Result<Box<dyn Backend>> {
        match self.account_config.backend.clone() {
            BackendConfig::None => Err(Error::BuildBackendError),
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !self.account_config.sync || self.disable_cache => {
                Ok(Box::new(ImapBackend::new(
                    self.account_config,
                    imap_config,
                    self.default_credentials,
                )?))
            }
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(_) => {
                let root_dir = self.account_config.sync_dir()?;
                Ok(Box::new(MaildirBackend::new(
                    self.account_config,
                    MaildirConfig { root_dir },
                )?))
            }
            BackendConfig::Maildir(mdir_config) => Ok(Box::new(MaildirBackend::new(
                self.account_config,
                mdir_config,
            )?)),
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(notmuch_config) => Ok(Box::new(NotmuchBackend::new(
                self.account_config,
                notmuch_config,
            )?)),
        }
    }
}
