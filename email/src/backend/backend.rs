//! Backend module.
//!
//! This module exposes the backend trait, which can be used to create
//! custom backend implementations.

use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use log::info;
use std::{any::Any, env, fmt, fs::OpenOptions, io, result};
use thiserror::Error;

#[cfg(feature = "imap-backend")]
use crate::ImapBackendBuilder;
use crate::{
    account, backend, email, envelope, folder, AccountConfig, BackendConfig, Emails, Envelope,
    Envelopes, Flag, Flags, Folders, MaildirBackend, MaildirConfig,
};

#[cfg(feature = "notmuch-backend")]
use crate::NotmuchBackend;

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
    ImapBackendError(#[from] backend::imap::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] backend::maildir::Error),
    #[cfg(feature = "notmuch-backend")]
    #[error(transparent)]
    NotmuchBackendError(#[from] backend::notmuch::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub trait Backend: Sync + Send {
    fn name(&self) -> String;

    fn add_folder(&self, folder: &str) -> Result<()>;
    fn list_folders(&self) -> Result<Folders>;
    fn expunge_folder(&self, folder: &str) -> Result<()>;
    fn purge_folder(&self, folder: &str) -> Result<()>;
    fn delete_folder(&self, folder: &str) -> Result<()>;

    fn get_envelope(&self, folder: &str, id: &str) -> Result<Envelope>;
    fn list_envelopes(&self, folder: &str, page_size: usize, page: usize) -> Result<Envelopes>;
    fn search_envelopes(
        &self,
        folder: &str,
        query: &str,
        sort: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;

    fn add_email(&self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;
    fn preview_emails(&self, folder: &str, ids: Vec<&str>) -> Result<Emails>;
    fn get_emails(&self, folder: &str, ids: Vec<&str>) -> Result<Emails>;
    fn copy_emails(&self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn move_emails(&self, from_folder: &str, to_folder: &str, ids: Vec<&str>) -> Result<()>;
    fn delete_emails(&self, folder: &str, ids: Vec<&str>) -> Result<()>;

    fn add_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn set_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;
    fn remove_flags(&self, folder: &str, ids: Vec<&str>, flags: &Flags) -> Result<()>;

    fn mark_emails_as_deleted(&self, folder: &str, ids: Vec<&str>) -> Result<()> {
        self.add_flags(folder, ids, &Flags::from_iter([Flag::Deleted]))
    }

    fn close(&self) -> Result<()> {
        Ok(())
    }

    // INFO: for downcasting purpose
    fn as_any(&self) -> &(dyn Any);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendSyncProgressEvent {
    GetLocalCachedFolders,
    GetLocalFolders,
    GetRemoteCachedFolders,
    GetRemoteFolders,
    BuildFoldersPatch,
    ProcessFoldersPatch(usize),
    ProcessFolderHunk(String),

    StartEnvelopesSync(String, usize, usize),
    GetLocalCachedEnvelopes,
    GetLocalEnvelopes,
    GetRemoteCachedEnvelopes,
    GetRemoteEnvelopes,
    BuildEnvelopesPatch,
    ProcessEnvelopesPatch(usize),
    ProcessEnvelopeHunk(String),
}

impl fmt::Display for BackendSyncProgressEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GetLocalCachedFolders => write!(f, "Getting local cached folders"),
            Self::GetLocalFolders => write!(f, "Getting local folders"),
            Self::GetRemoteCachedFolders => write!(f, "Getting remote cached folders"),
            Self::GetRemoteFolders => write!(f, "Getting remote folders"),
            Self::BuildFoldersPatch => write!(f, "Building folders patch"),
            Self::ProcessFoldersPatch(n) => write!(f, "Processing {n} hunks of folders patch"),
            Self::ProcessFolderHunk(s) => write!(f, "Processing folder hunk: {s}"),

            Self::StartEnvelopesSync(_, _, _) => write!(f, "Starting envelopes synchronization"),
            Self::GetLocalCachedEnvelopes => write!(f, "Getting local cached envelopes"),
            Self::GetLocalEnvelopes => write!(f, "Getting local envelopes"),
            Self::GetRemoteCachedEnvelopes => write!(f, "Getting remote cached envelopes"),
            Self::GetRemoteEnvelopes => write!(f, "Getting remote envelopes"),
            Self::BuildEnvelopesPatch => write!(f, "Building envelopes patch"),
            Self::ProcessEnvelopesPatch(n) => write!(f, "Processing {n} hunks of envelopes patch"),
            Self::ProcessEnvelopeHunk(s) => write!(f, "Processing envelope hunk: {s}"),
        }
    }
}

#[derive(Debug, Default)]
pub struct BackendSyncReport {
    pub folders: folder::sync::FoldersName,
    pub folders_patch: Vec<(folder::sync::Hunk, Option<folder::sync::Error>)>,
    pub folders_cache_patch: (Vec<folder::sync::CacheHunk>, Option<folder::sync::Error>),
    pub envelopes_patch: Vec<(envelope::sync::BackendHunk, Option<envelope::sync::Error>)>,
    pub envelopes_cache_patch: (Vec<envelope::sync::CacheHunk>, Vec<envelope::sync::Error>),
}

pub struct BackendSyncBuilder<'a> {
    account_config: &'a AccountConfig,
    on_progress: Box<dyn Fn(BackendSyncProgressEvent) -> Result<()> + Sync + Send + 'a>,
    folders_strategy: folder::sync::Strategy,
    dry_run: bool,
}

impl<'a> BackendSyncBuilder<'a> {
    pub fn new(account_config: &'a AccountConfig) -> Self {
        Self {
            account_config,
            on_progress: Box::new(|_| Ok(())),
            folders_strategy: account_config.sync_folders_strategy.clone(),
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

    pub fn folders_strategy(mut self, strategy: folder::sync::Strategy) -> Self {
        self.folders_strategy = strategy;
        self
    }

    pub fn dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn sync(&self, remote: &dyn Backend) -> Result<BackendSyncReport> {
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

        let mut conn = rusqlite::Connection::open(sync_dir.join(".sync.sqlite"))?;

        folder::sync::Cache::init(&mut conn)?;
        envelope::sync::Cache::init(&mut conn)?;

        // init local Maildir

        let local = MaildirBackend::new(
            self.account_config.clone(),
            MaildirConfig {
                root_dir: sync_dir.clone(),
            },
        )?;

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

        let folders_sync_report = folder::SyncBuilder::new(self.account_config)
            .on_progress(|data| Ok(progress(data).map_err(Box::new)?))
            .strategy(folders_strategy)
            .dry_run(self.dry_run)
            .sync(&mut conn, &local, remote)?;

        let envelopes = envelope::SyncBuilder::new(self.account_config)
            .on_progress(|data| Ok(progress(data).map_err(Box::new)?))
            .dry_run(self.dry_run);

        let mut envelopes_patch = Vec::new();
        let mut envelopes_cache_patch = (Vec::new(), Vec::new());

        for (folder_num, folder) in folders_sync_report.folders.iter().enumerate() {
            progress(BackendSyncProgressEvent::StartEnvelopesSync(
                folder.clone(),
                folder_num + 1,
                folders_sync_report.folders.len(),
            ))?;
            let report = envelopes.sync(folder, &mut conn, &local, remote)?;
            envelopes_patch.extend(report.patch);
            envelopes_cache_patch.0.extend(report.cache_patch.0);
            if let Some(err) = report.cache_patch.1 {
                envelopes_cache_patch.1.push(err);
            }

            local.expunge_folder(folder)?;
            remote.expunge_folder(folder)?;
        }

        lock_file
            .unlock()
            .map_err(|err| Error::SyncAccountUnlockFileError(err, account.clone()))?;

        Ok(BackendSyncReport {
            folders: folders_sync_report.folders,
            folders_patch: folders_sync_report.patch,
            folders_cache_patch: folders_sync_report.cache_patch,
            envelopes_patch,
            envelopes_cache_patch,
        })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendBuilder {
    sessions_pool_size: usize,
    disable_cache: bool,
}

impl BackendBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sessions_pool_size(mut self, pool_size: usize) -> Self {
        self.sessions_pool_size = pool_size;
        self
    }

    pub fn disable_cache(mut self, disable_cache: bool) -> Self {
        self.disable_cache = disable_cache;
        self
    }

    pub fn build(
        &self,
        account_config: AccountConfig,
        backend_config: BackendConfig,
    ) -> Result<Box<dyn Backend>> {
        match backend_config {
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(imap_config) if !account_config.sync || self.disable_cache => {
                Ok(Box::new(
                    ImapBackendBuilder::new()
                        .pool_size(self.sessions_pool_size)
                        .build(account_config, imap_config)?,
                ))
            }
            #[cfg(feature = "imap-backend")]
            BackendConfig::Imap(_) => {
                let root_dir = account_config.sync_dir()?;
                Ok(Box::new(MaildirBackend::new(
                    account_config,
                    MaildirConfig { root_dir },
                )?))
            }
            BackendConfig::Maildir(maildir_config) => Ok(Box::new(MaildirBackend::new(
                account_config,
                maildir_config,
            )?)),
            #[cfg(feature = "notmuch-backend")]
            BackendConfig::Notmuch(notmuch_config) => Ok(Box::new(NotmuchBackend::new(
                account_config,
                notmuch_config,
            )?)),
            BackendConfig::None => Err(Error::BuildBackendError),
        }
    }
}
