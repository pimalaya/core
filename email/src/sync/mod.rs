use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use futures::Future;
use log::debug;
use std::{env, fmt, fs::OpenOptions, io, path::PathBuf, sync::Arc};
use thiserror::Error;

use crate::{
    backend::{BackendBuilder, BackendContextBuilder},
    folder::sync::{FolderSyncBuilder, FolderSyncEvent, FolderSyncEventHandler, FolderSyncReport},
    Result,
};

/// Errors related to synchronization.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot open sync lock file")]
    OpenLockFileError(#[source] io::Error, PathBuf),
    #[error("cannot lock sync file")]
    LockFileError(#[source] FileLockError, PathBuf),
    #[error("cannot unlock sync file")]
    UnlockFileError(#[source] FileLockError, PathBuf),
}

/// The synchronization destination.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
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

#[derive(Clone)]
pub struct SyncBuilder<L: BackendContextBuilder, R: BackendContextBuilder> {
    id: String,
    left_builder: BackendBuilder<L>,
    right_builder: BackendBuilder<R>,
    folder_handler: Option<Arc<FolderSyncEventHandler>>,
    cache_dir: Option<PathBuf>,
}

impl<L: BackendContextBuilder + 'static, R: BackendContextBuilder + 'static> SyncBuilder<L, R> {
    pub fn new(left_builder: BackendBuilder<L>, right_builder: BackendBuilder<R>) -> Self {
        let id = left_builder.account_config.name.clone() + &right_builder.account_config.name;
        let id = format!("{:x}", md5::compute(id));

        Self {
            id,
            left_builder,
            right_builder,
            folder_handler: None,
            cache_dir: None,
        }
    }

    pub fn set_some_folder_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: Option<impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static>,
    ) {
        self.folder_handler = match handler {
            Some(handler) => Some(Arc::new(move |evt| Box::pin(handler(evt)))),
            None => None,
        };
    }

    pub fn set_folder_handler<F: Future<Output = Result<()>> + Send + 'static>(
        &mut self,
        handler: impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static,
    ) {
        self.set_some_folder_handler(Some(handler));
    }

    pub fn with_some_folder_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: Option<impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static>,
    ) -> Self {
        self.set_some_folder_handler(handler);
        self
    }

    pub fn with_folder_handler<F: Future<Output = Result<()>> + Send + 'static>(
        mut self,
        handler: impl Fn(FolderSyncEvent) -> F + Send + Sync + 'static,
    ) -> Self {
        self.set_folder_handler(handler);
        self
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

    pub async fn sync(self) -> Result<FolderSyncReport> {
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

        let folder_sync_report = FolderSyncBuilder::new(self.left_builder, self.right_builder)
            .with_some_atomic_handler_ref(self.folder_handler)
            .with_some_cache_dir(self.cache_dir)
            .sync()
            .await?;

        debug!("unlocking sync file");
        lock_file
            .unlock()
            .map_err(|err| Error::UnlockFileError(err, lock_file_path))?;

        Ok(folder_sync_report)
    }
}
