//! # Synchronization
//!
//! Module dedicated to synchronization of folders and emails between
//! two backends. The main structure of this module is
//! [`SyncBuilder`].

pub mod pool;
pub mod report;

use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use dirs::cache_dir;
use log::{debug, trace};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fmt,
    fs::OpenOptions,
    future::Future,
    io,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};
use thiserror::Error;

use crate::{
    backend::{context::BackendContextBuilder, BackendBuilder},
    email::{self, sync::hunk::EmailSyncHunk},
    folder::{
        self,
        sync::{
            hunk::{FolderName, FolderSyncHunk},
            patch::FolderSyncPatch,
            FolderSyncStrategy,
        },
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    Result,
};

use self::report::SyncReport;

/// Errors related to synchronization.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open sync lock file")]
    OpenLockFileError(#[source] io::Error, PathBuf),
    #[error("cannot lock sync file")]
    LockFileError(#[source] FileLockError, PathBuf),
    #[error("cannot unlock sync file")]
    UnlockFileError(#[source] FileLockError, PathBuf),

    #[error("cannot get sync cache directory")]
    GetCacheDirectoryError,
}

/// The synchronization builder.
#[derive(Clone)]
pub struct SyncBuilder<L: BackendContextBuilder, R: BackendContextBuilder> {
    id: String,
    left_builder: BackendBuilder<L>,
    right_builder: BackendBuilder<R>,
    cache_dir: Option<PathBuf>,
    handler: Option<Arc<SyncEventHandler>>,
    dry_run: Option<bool>,
    filters: Option<SyncFilters>,
}

impl<L: BackendContextBuilder + 'static, R: BackendContextBuilder + 'static> SyncBuilder<L, R> {
    /// Create a new synchronization builder using the two given
    /// backend builders.
    pub fn new(left_builder: BackendBuilder<L>, right_builder: BackendBuilder<R>) -> Self {
        let id = left_builder.account_config.name.clone() + &right_builder.account_config.name;
        let id = format!("{:x}", md5::compute(id));

        Self {
            id,
            left_builder,
            right_builder,
            cache_dir: None,
            handler: None,
            dry_run: None,
            filters: None,
        }
    }

    pub fn set_some_cache_dir(&mut self, dir: Option<impl Into<PathBuf>>) {
        self.cache_dir = dir.map(Into::into);
    }

    pub fn set_cache_dir(&mut self, dir: impl Into<PathBuf>) {
        self.set_some_cache_dir(Some(dir));
    }

    pub fn with_some_cache_dir(mut self, dir: Option<impl Into<PathBuf>>) -> Self {
        self.set_some_cache_dir(dir);
        self
    }

    pub fn with_cache_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.set_cache_dir(dir);
        self
    }

    pub fn find_default_cache_dir(&self) -> Option<PathBuf> {
        cache_dir().map(|dir| {
            dir.join("pimalaya")
                .join("email")
                .join("sync")
                .join(&self.id)
        })
    }

    pub fn find_cache_dir(&self) -> Option<PathBuf> {
        self.cache_dir
            .as_ref()
            .cloned()
            .or_else(|| self.find_default_cache_dir())
    }

    pub fn get_cache_dir(&self) -> Result<PathBuf> {
        self.find_cache_dir()
            .ok_or(Error::GetCacheDirectoryError.into())
    }

    pub fn set_some_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: Option<impl Fn(SyncEvent) -> F + Send + Sync + 'static>,
    ) {
        self.handler = match handler {
            Some(handler) => Some(Arc::new(move |evt| Box::pin(handler(evt)))),
            None => None,
        };
    }

    pub fn set_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: impl Fn(SyncEvent) -> F + Send + Sync + 'static,
    ) {
        self.set_some_handler(Some(handler));
    }

    pub fn with_some_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: Option<impl Fn(SyncEvent) -> F + Send + Sync + 'static>,
    ) -> Self {
        self.set_some_handler(handler);
        self
    }

    pub fn with_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: impl Fn(SyncEvent) -> F + Send + Sync + 'static,
    ) -> Self {
        self.set_handler(handler);
        self
    }

    pub fn set_some_dry_run(&mut self, dry_run: Option<bool>) {
        self.dry_run = dry_run;
    }

    pub fn set_dry_run(&mut self, dry_run: bool) {
        self.set_some_dry_run(Some(dry_run));
    }

    pub fn with_some_dry_run(mut self, dry_run: Option<bool>) -> Self {
        self.set_some_dry_run(dry_run);
        self
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.set_dry_run(dry_run);
        self
    }

    pub fn get_dry_run(&self) -> bool {
        self.dry_run.unwrap_or_default()
    }

    pub fn set_some_folders_filter(&mut self, folders: Option<impl Into<FolderSyncStrategy>>) {
        let folders = folders.map(Into::into);
        match self.filters.as_mut() {
            Some(filters) => filters.folders = folders,
            None => self.filters = Some(SyncFilters { folders }),
        }
    }

    pub fn set_folders_filter(&mut self, folders: impl Into<FolderSyncStrategy>) {
        self.set_some_folders_filter(Some(folders));
    }

    pub fn with_some_folders_filter(
        mut self,
        folders: Option<impl Into<FolderSyncStrategy>>,
    ) -> Self {
        self.set_some_folders_filter(folders);
        self
    }

    pub fn with_folders_filter(mut self, folders: impl Into<FolderSyncStrategy>) -> Self {
        self.set_folders_filter(folders);
        self
    }

    pub fn get_folders_filter(&self) -> FolderSyncStrategy {
        self.filters
            .as_ref()
            .and_then(|f| f.folders.as_ref())
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_left_cache_builder(&self) -> Result<BackendBuilder<MaildirContextBuilder>> {
        let left_config = self.left_builder.account_config.clone();
        let root_dir = self.get_cache_dir()?.join(&left_config.name);
        let ctx =
            MaildirContextBuilder::new(left_config.clone(), Arc::new(MaildirConfig { root_dir }));
        let left_cache_builder = BackendBuilder::new(left_config, ctx);
        Ok(left_cache_builder)
    }

    pub fn get_right_cache_builder(&self) -> Result<BackendBuilder<MaildirContextBuilder>> {
        let right_config = self.right_builder.account_config.clone();
        let root_dir = self.get_cache_dir()?.join(&right_config.name);
        let ctx =
            MaildirContextBuilder::new(right_config.clone(), Arc::new(MaildirConfig { root_dir }));
        let right_cache_builder = BackendBuilder::new(right_config, ctx);
        Ok(right_cache_builder)
    }

    pub async fn sync(self) -> Result<SyncReport> {
        let lock_file_name = format!("pimalaya-email-sync.{}.lock", self.id);
        let lock_file_path = env::temp_dir().join(lock_file_name);

        debug!("locking sync file {lock_file_path:?}");
        let lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&lock_file_path)
            .map_err(|err| Error::OpenLockFileError(err, lock_file_path.clone()))?;
        lock_file
            .try_lock(FileLockMode::Exclusive)
            .map_err(|err| Error::LockFileError(err, lock_file_path.clone()))?;

        let pool = Arc::new(
            pool::new(
                self.get_left_cache_builder()?,
                self.left_builder.clone(),
                self.get_right_cache_builder()?,
                self.right_builder.clone(),
                self.handler.clone(),
                self.get_dry_run(),
                self.get_folders_filter(),
            )
            .await?,
        );

        let mut report = SyncReport::default();

        report.folder = folder::sync::<L, R>(pool.clone()).await?;
        report.email = email::sync::<L, R>(pool.clone(), &report.folder.names).await?;

        folder::sync::expunge::<L, R>(pool.clone(), &report.folder.names).await;

        Arc::into_inner(pool).unwrap().close().await;

        debug!("unlocking sync file");
        lock_file
            .unlock()
            .map_err(|err| Error::UnlockFileError(err, lock_file_path))?;

        Ok(report)
    }
}

/// The synchronization async event handler.
pub type SyncEventHandler =
    dyn Fn(SyncEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync;

/// The synchronization event.
///
/// Represents all the events that can be triggered during the
/// backends synchronization process.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum SyncEvent {
    ListedLeftCachedFolders(usize),
    ListedLeftFolders(usize),
    ListedRightCachedFolders(usize),
    ListedRightFolders(usize),
    ListedAllFolders,
    GeneratedFolderPatch(BTreeMap<FolderName, FolderSyncPatch>),
    ProcessedFolderHunk(FolderSyncHunk),
    ProcessedAllFolderHunks,
    ListedLeftCachedEnvelopes(FolderName, usize),
    ListedLeftEnvelopes(FolderName, usize),
    ListedRightCachedEnvelopes(FolderName, usize),
    ListedRightEnvelopes(FolderName, usize),
    GeneratedEmailPatch(BTreeMap<FolderName, BTreeSet<EmailSyncHunk>>),
    ProcessedEmailHunk(EmailSyncHunk),
    ProcessedAllEmailHunks,
    ExpungedAllFolders,
}

impl SyncEvent {
    pub async fn emit(&self, handler: &Option<Arc<SyncEventHandler>>) {
        if let Some(handler) = handler.as_ref() {
            if let Err(err) = handler(self.clone()).await {
                debug!("error while emitting sync event: {err}");
                trace!("{err:?}");
            } else {
                debug!("emitted sync event {self:?}");
            }
        }
    }
}

impl fmt::Display for SyncEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncEvent::ListedLeftCachedFolders(n) => {
                write!(f, "Listed {n} left cached folders")
            }
            SyncEvent::ListedLeftFolders(n) => {
                write!(f, "Listed {n} left folders")
            }
            SyncEvent::ListedRightCachedFolders(n) => {
                write!(f, "Listed {n} right cached folders")
            }
            SyncEvent::ListedRightFolders(n) => {
                write!(f, "Listed {n} right folders")
            }
            SyncEvent::ListedAllFolders => {
                write!(f, "Listed all folders")
            }
            SyncEvent::GeneratedFolderPatch(patch) => {
                let n = patch.keys().count();
                let p = patch.values().flatten().count();
                write!(f, "Generated {p} patch for {n} folders")
            }
            SyncEvent::ProcessedFolderHunk(hunk) => {
                write!(f, "{hunk}")
            }
            SyncEvent::ProcessedAllFolderHunks => {
                write!(f, "Processed all folder hunks")
            }
            SyncEvent::ListedLeftCachedEnvelopes(folder, n) => {
                write!(f, "Listed {n} left cached envelopes from {folder}")
            }
            SyncEvent::ListedLeftEnvelopes(folder, n) => {
                write!(f, "Listed {n} left envelopes from {folder}")
            }
            SyncEvent::ListedRightCachedEnvelopes(folder, n) => {
                write!(f, "Listed {n} right cached envelopes from {folder}")
            }
            SyncEvent::ListedRightEnvelopes(folder, n) => {
                write!(f, "Listed {n} right envelopes from {folder}")
            }
            SyncEvent::GeneratedEmailPatch(patch) => {
                let nf = patch.keys().count();
                let np = patch.values().flatten().count();
                write!(f, "Generated {np} patch for {nf} folders")
            }
            SyncEvent::ProcessedEmailHunk(hunk) => {
                write!(f, "{hunk}")
            }
            SyncEvent::ProcessedAllEmailHunks => {
                write!(f, "Processed all email hunks")
            }
            SyncEvent::ExpungedAllFolders => {
                write!(f, "Expunged all folders")
            }
        }
    }
}

/// The synchronization destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum SyncDestination {
    Left,
    Right,
}

impl fmt::Display for SyncDestination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => write!(f, "left"),
            Self::Right => write!(f, "right"),
        }
    }
}

/// The synchronization filters.
#[derive(Clone)]
pub struct SyncFilters {
    /// Filter folders using the given strategy.
    folders: Option<FolderSyncStrategy>,
}
