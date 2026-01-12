//! # Synchronization
//!
//! Module dedicated to synchronization of folders and emails between
//! two backends. The main structure of this module is
//! [`SyncBuilder`].

mod error;
pub mod hash;
pub mod pool;
pub mod report;

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fmt,
    fs::{self, OpenOptions},
    future::Future,
    hash::{DefaultHasher, Hash, Hasher},
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};

use advisory_lock::{AdvisoryFileLock, FileLockMode};
use dirs::{cache_dir, runtime_dir};
use once_cell::sync::Lazy;
use tracing::debug;

#[doc(inline)]
pub use self::error::{Error, Result};
use self::{hash::SyncHash, report::SyncReport};
use crate::{
    backend::{context::BackendContextBuilder, BackendBuilder},
    email::{self, sync::hunk::EmailSyncHunk},
    envelope::sync::config::EnvelopeSyncFilters,
    flag::sync::config::FlagSyncPermissions,
    folder::{
        self,
        sync::{
            config::{FolderSyncPermissions, FolderSyncStrategy},
            hunk::{FolderName, FolderSyncHunk},
            patch::FolderSyncPatch,
        },
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder},
    message::sync::config::MessageSyncPermissions,
    sync::pool::{SyncPoolConfig, SyncPoolContextBuilder},
};

static RUNTIME_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let dir = runtime_dir()
        .unwrap_or_else(env::temp_dir)
        .join("pimalaya")
        .join("email")
        .join("sync");
    fs::create_dir_all(&dir).expect(&format!("should create runtime directory {dir:?}"));
    dir
});

/// The synchronization builder.
#[derive(Clone)]
pub struct SyncBuilder<L: BackendContextBuilder + SyncHash, R: BackendContextBuilder + SyncHash> {
    config: SyncPoolConfig,
    left_builder: BackendBuilder<L>,
    left_hash: String,
    right_builder: BackendBuilder<R>,
    right_hash: String,
    cache_dir: Option<PathBuf>,
}

impl<L, R> SyncBuilder<L, R>
where
    L: BackendContextBuilder + SyncHash + 'static,
    R: BackendContextBuilder + SyncHash + 'static,
{
    /// Create a new synchronization builder using the two given
    /// backend builders.
    pub fn new(left_builder: BackendBuilder<L>, right_builder: BackendBuilder<R>) -> Self {
        let mut left_hasher = DefaultHasher::new();
        left_builder.sync_hash(&mut left_hasher);
        let left_hash = format!("{:x}", left_hasher.finish());

        let mut right_hasher = DefaultHasher::new();
        right_builder.sync_hash(&mut right_hasher);
        let right_hash = format!("{:x}", right_hasher.finish());

        Self {
            config: Default::default(),
            left_builder,
            left_hash,
            right_builder,
            right_hash,
            cache_dir: None,
        }
    }

    // cache dir setters

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

    // handler setters

    pub fn set_some_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: Option<impl Fn(SyncEvent) -> F + Send + Sync + 'static>,
    ) {
        self.config.handler = match handler {
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

    // dry run setters and getter

    pub fn set_some_dry_run(&mut self, dry_run: Option<bool>) {
        self.config.dry_run = dry_run;
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
        self.config.dry_run.unwrap_or_default()
    }

    // folder filters setters

    pub fn set_some_folder_filters(&mut self, f: Option<impl Into<FolderSyncStrategy>>) {
        self.config.folder_filters = f.map(Into::into);
    }

    pub fn set_folder_filters(&mut self, f: impl Into<FolderSyncStrategy>) {
        self.set_some_folder_filters(Some(f));
    }

    pub fn with_some_folder_filters(mut self, f: Option<impl Into<FolderSyncStrategy>>) -> Self {
        self.set_some_folder_filters(f);
        self
    }

    pub fn with_folder_filters(mut self, f: impl Into<FolderSyncStrategy>) -> Self {
        self.set_folder_filters(f);
        self
    }

    // left folder permissions setters

    pub fn set_some_left_folder_permissions(
        &mut self,
        p: Option<impl Into<FolderSyncPermissions>>,
    ) {
        self.config.left_folder_permissions = p.map(Into::into);
    }

    pub fn set_left_folder_permissions(&mut self, p: impl Into<FolderSyncPermissions>) {
        self.set_some_left_folder_permissions(Some(p));
    }

    pub fn with_some_left_folder_permissions(
        mut self,
        p: Option<impl Into<FolderSyncPermissions>>,
    ) -> Self {
        self.set_some_left_folder_permissions(p);
        self
    }

    pub fn with_left_folder_permissions(mut self, p: impl Into<FolderSyncPermissions>) -> Self {
        self.set_left_folder_permissions(p);
        self
    }

    // right folder permissions setters

    pub fn set_some_right_folder_permissions(
        &mut self,
        p: Option<impl Into<FolderSyncPermissions>>,
    ) {
        self.config.right_folder_permissions = p.map(Into::into);
    }

    pub fn set_right_folder_permissions(&mut self, p: impl Into<FolderSyncPermissions>) {
        self.set_some_right_folder_permissions(Some(p));
    }

    pub fn with_some_right_folder_permissions(
        mut self,
        p: Option<impl Into<FolderSyncPermissions>>,
    ) -> Self {
        self.set_some_right_folder_permissions(p);
        self
    }

    pub fn with_right_folder_permissions(mut self, p: impl Into<FolderSyncPermissions>) -> Self {
        self.set_right_folder_permissions(p);
        self
    }

    // envelope filters setters

    pub fn set_some_envelope_filters(&mut self, f: Option<impl Into<EnvelopeSyncFilters>>) {
        self.config.envelope_filters = f.map(Into::into);
    }

    pub fn set_envelope_filters(&mut self, f: impl Into<EnvelopeSyncFilters>) {
        self.set_some_envelope_filters(Some(f));
    }

    pub fn with_some_envelope_filters(mut self, f: Option<impl Into<EnvelopeSyncFilters>>) -> Self {
        self.set_some_envelope_filters(f);
        self
    }

    pub fn with_envelope_filters(mut self, f: impl Into<EnvelopeSyncFilters>) -> Self {
        self.set_envelope_filters(f);
        self
    }

    // left flag permissions setters

    pub fn set_some_left_flag_permissions(&mut self, p: Option<impl Into<FlagSyncPermissions>>) {
        self.config.left_flag_permissions = p.map(Into::into);
    }

    pub fn set_left_flag_permissions(&mut self, p: impl Into<FlagSyncPermissions>) {
        self.set_some_left_flag_permissions(Some(p));
    }

    pub fn with_some_left_flag_permissions(
        mut self,
        p: Option<impl Into<FlagSyncPermissions>>,
    ) -> Self {
        self.set_some_left_flag_permissions(p);
        self
    }

    pub fn with_left_flag_permissions(mut self, p: impl Into<FlagSyncPermissions>) -> Self {
        self.set_left_flag_permissions(p);
        self
    }

    // right flag permissions setters

    pub fn set_some_right_flag_permissions(&mut self, p: Option<impl Into<FlagSyncPermissions>>) {
        self.config.right_flag_permissions = p.map(Into::into);
    }

    pub fn set_right_flag_permissions(&mut self, p: impl Into<FlagSyncPermissions>) {
        self.set_some_right_flag_permissions(Some(p));
    }

    pub fn with_some_right_flag_permissions(
        mut self,
        p: Option<impl Into<FlagSyncPermissions>>,
    ) -> Self {
        self.set_some_right_flag_permissions(p);
        self
    }

    pub fn with_right_flag_permissions(mut self, p: impl Into<FlagSyncPermissions>) -> Self {
        self.set_right_flag_permissions(p);
        self
    }

    // left message permissions setters

    pub fn set_some_left_message_permissions(
        &mut self,
        p: Option<impl Into<MessageSyncPermissions>>,
    ) {
        self.config.left_message_permissions = p.map(Into::into);
    }

    pub fn set_left_message_permissions(&mut self, p: impl Into<MessageSyncPermissions>) {
        self.set_some_left_message_permissions(Some(p));
    }

    pub fn with_some_left_message_permissions(
        mut self,
        p: Option<impl Into<MessageSyncPermissions>>,
    ) -> Self {
        self.set_some_left_message_permissions(p);
        self
    }

    pub fn with_left_message_permissions(mut self, p: impl Into<MessageSyncPermissions>) -> Self {
        self.set_left_message_permissions(p);
        self
    }

    // right message permissions setters

    pub fn set_some_right_message_permissions(
        &mut self,
        p: Option<impl Into<MessageSyncPermissions>>,
    ) {
        self.config.right_message_permissions = p.map(Into::into);
    }

    pub fn set_right_message_permissions(&mut self, p: impl Into<MessageSyncPermissions>) {
        self.set_some_right_message_permissions(Some(p));
    }

    pub fn with_some_right_message_permissions(
        mut self,
        p: Option<impl Into<MessageSyncPermissions>>,
    ) -> Self {
        self.set_some_right_message_permissions(p);
        self
    }

    pub fn with_right_message_permissions(mut self, p: impl Into<MessageSyncPermissions>) -> Self {
        self.set_right_message_permissions(p);
        self
    }

    // getters

    pub fn find_default_cache_dir(&self) -> Option<PathBuf> {
        cache_dir().map(|dir| dir.join("pimalaya").join("email").join("sync"))
    }

    pub fn get_cache_dir(&self) -> Result<PathBuf> {
        self.cache_dir
            .as_ref()
            .cloned()
            .or_else(|| self.find_default_cache_dir())
            .ok_or(Error::GetCacheDirectorySyncError.into())
    }

    pub fn get_left_cache_builder(&self) -> Result<BackendBuilder<MaildirContextBuilder>> {
        let left_config = self.left_builder.account_config.clone();
        let root_dir = self.get_cache_dir()?.join(&self.left_hash);
        let ctx = MaildirContextBuilder::new(
            left_config.clone(),
            Arc::new(MaildirConfig {
                root_dir,
                maildirpp: false,
            }),
        );
        let left_cache_builder = BackendBuilder::new(left_config, ctx);
        Ok(left_cache_builder)
    }

    pub fn get_right_cache_builder(&self) -> Result<BackendBuilder<MaildirContextBuilder>> {
        let right_config = self.right_builder.account_config.clone();
        let root_dir = self.get_cache_dir()?.join(&self.right_hash);
        let ctx = MaildirContextBuilder::new(
            right_config.clone(),
            Arc::new(MaildirConfig {
                root_dir,
                maildirpp: false,
            }),
        );
        let right_cache_builder = BackendBuilder::new(right_config, ctx);
        Ok(right_cache_builder)
    }

    // build

    pub async fn sync(self) -> Result<SyncReport> {
        let left_lock_file_path = RUNTIME_DIR.join(format!("{}.lock", self.left_hash));
        debug!("locking left sync file {left_lock_file_path:?}");
        let left_lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&left_lock_file_path)
            .map_err(|err| Error::OpenLockFileError(err, left_lock_file_path.clone()))?;
        AdvisoryFileLock::try_lock(&left_lock_file, FileLockMode::Exclusive)
            .map_err(|err| Error::LockFileError(err, left_lock_file_path.clone()))?;

        let right_lock_file_path = RUNTIME_DIR.join(format!("{}.lock", self.right_hash));
        debug!("locking right sync file {right_lock_file_path:?}");
        let right_lock_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&right_lock_file_path)
            .map_err(|err| Error::OpenLockFileError(err, right_lock_file_path.clone()))?;
        AdvisoryFileLock::try_lock(&right_lock_file, FileLockMode::Exclusive)
            .map_err(|err| Error::LockFileError(err, right_lock_file_path.clone()))?;

        let mut left_cache_builder = self.get_left_cache_builder()?;
        let left_cache_check = left_cache_builder.ctx_builder.check_configuration();

        let mut left_builder = self.left_builder.clone();
        let left_check = left_builder.ctx_builder.check_configuration();

        match (left_cache_check, left_check) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(err)) => Err(Error::LeftContextNotConfiguredError(err)),
            (Err(_), Ok(())) => {
                left_cache_builder
                    .ctx_builder
                    .configure()
                    .await
                    .map_err(Error::ConfigureLeftContextError)?;
                Ok(())
            }
            (Err(_), Err(_)) => {
                left_cache_builder
                    .ctx_builder
                    .configure()
                    .await
                    .map_err(Error::ConfigureLeftContextError)?;
                left_builder
                    .ctx_builder
                    .configure()
                    .await
                    .map_err(Error::ConfigureLeftContextError)?;
                Ok(())
            }
        }?;

        let mut right_cache_builder = self.get_right_cache_builder()?;
        let right_cache_check = right_cache_builder.ctx_builder.check_configuration();

        let mut right_builder = self.right_builder.clone();
        let right_check = right_builder.ctx_builder.check_configuration();

        match (right_cache_check, right_check) {
            (Ok(()), Ok(())) => Ok(()),
            (Ok(()), Err(err)) => Err(Error::RightContextNotConfiguredError(err)),
            (Err(_), Ok(())) => {
                right_cache_builder
                    .ctx_builder
                    .configure()
                    .await
                    .map_err(Error::ConfigureRightContextError)?;
                Ok(())
            }
            (Err(_), Err(_)) => {
                right_cache_builder
                    .ctx_builder
                    .configure()
                    .await
                    .map_err(Error::ConfigureRightContextError)?;
                right_builder
                    .ctx_builder
                    .configure()
                    .await
                    .map_err(Error::ConfigureRightContextError)?;
                Ok(())
            }
        }?;

        let ctx = Arc::new(
            SyncPoolContextBuilder::new(
                self.config,
                left_cache_builder,
                left_builder,
                right_cache_builder,
                right_builder,
            )
            .build()
            .await
            .map_err(Error::BuildSyncPoolContextError)?,
        );

        let mut report = SyncReport::default();

        report.folder = folder::sync::<L, R>(ctx.clone())
            .await
            .map_err(Error::SyncFoldersError)?;
        report.email = email::sync::<L, R>(ctx.clone(), &report.folder.names)
            .await
            .map_err(Error::SyncEmailsError)?;

        folder::sync::expunge::<L, R>(ctx.clone(), &report.folder.names).await;

        debug!("unlocking sync files");
        AdvisoryFileLock::unlock(&left_lock_file)
            .map_err(|err| Error::UnlockFileError(err, left_lock_file_path))?;
        AdvisoryFileLock::unlock(&right_lock_file)
            .map_err(|err| Error::UnlockFileError(err, right_lock_file_path))?;

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
                debug!(?err, "error while emitting sync event");
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
