pub mod report;

use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use futures::{lock::Mutex, Future};
use log::debug;
use std::{
    collections::HashSet, env, fmt, fs::OpenOptions, io, path::PathBuf, pin::Pin, sync::Arc,
};
use thiserror::Error;
use tokio::sync::mpsc;

use crate::{
    backend::{Backend, BackendBuilder, BackendContext, BackendContextBuilder},
    folder::Folder,
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
    handler: Option<Arc<SyncEventHandler>>,
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
            handler: None,
            cache_dir: None,
        }
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
        dirs::cache_dir().map(|dir| {
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

        let report = SyncReport::default();

        enum SyncTask {
            ListLeftFolders(HashSet<String>),
            ListRightFolders(HashSet<String>),
        }

        let mut pool =
            ThreadPoolBuilder::new(self.left_builder.clone(), self.right_builder.clone())
                .build()
                .await?;

        let handler = self.handler.clone();
        pool.send(|backends| async move {
            let folders = backends.left.list_folders().await?;

            let names = HashSet::<String>::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedLeftFolders(names.len())
                .emit(&handler)
                .await;

            Result::Ok(SyncTask::ListLeftFolders(names))
        })
        .await;

        let handler = self.handler.clone();
        pool.send(|backends| async move {
            let folders = backends.right.list_folders().await?;

            let names = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedRightFolders(names.len())
                .emit(&handler)
                .await;

            Result::Ok(SyncTask::ListRightFolders(names))
        })
        .await;

        let mut left_folders = None::<HashSet<String>>;
        let mut right_folders = None::<HashSet<String>>;

        while left_folders.is_none() || right_folders.is_none() {
            match pool.recv().await {
                None => break,
                Some(Err(err)) => Err(err)?,
                Some(Ok(SyncTask::ListLeftFolders(names))) => {
                    left_folders = Some(names);
                }
                Some(Ok(SyncTask::ListRightFolders(names))) => {
                    right_folders = Some(names);
                }
            }
        }

        println!("left_folders: {:#?}", left_folders);
        println!("right_folders: {:#?}", right_folders);

        // report.folder =
        //     FolderSyncBuilder::new(self.left_builder.clone(), self.right_builder.clone())
        //         .with_some_atomic_handler_ref(self.folder_handler)
        //         .with_some_cache_dir(self.cache_dir.clone())
        //         .sync()
        //         .await?;

        // let email_sync_builder = EmailSyncBuilder::new(self.left_builder, self.right_builder)
        //     .with_some_atomic_handler_ref(self.email_handler)
        //     .with_some_cache_dir(self.cache_dir);

        // for folder in &report.folder.folders {
        //     let email_sync_report = email_sync_builder.clone().sync(folder).await?;
        //     report.email.patch.extend(email_sync_report.patch);
        // }

        debug!("unlocking sync file");
        lock_file
            .unlock()
            .map_err(|err| Error::UnlockFileError(err, lock_file_path))?;

        Ok(report)
    }
}

type Task<L, R, T> = Box<
    dyn FnOnce(Arc<SyncBackends<L, R>>) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync,
>;

#[derive(Clone)]
struct ThreadPoolBuilder<L: BackendContextBuilder, R: BackendContextBuilder> {
    left_builder: BackendBuilder<L>,
    right_builder: BackendBuilder<R>,
    size: usize,
}

impl<L: BackendContextBuilder + 'static, R: BackendContextBuilder + 'static>
    ThreadPoolBuilder<L, R>
{
    pub fn new(left_builder: BackendBuilder<L>, right_builder: BackendBuilder<R>) -> Self {
        Self {
            left_builder,
            right_builder,
            size: 8,
        }
    }

    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub async fn build<T: Send + 'static>(self) -> Result<ThreadPool<L::Context, R::Context, T>> {
        let (tx_worker, rx) = mpsc::channel(1);
        let (tx, rx_worker) = mpsc::channel::<Task<L::Context, R::Context, T>>(1);
        let rx_worker = Arc::new(Mutex::new(rx_worker));

        for id in 0..self.size {
            println!("worker {id} init");
            let tx = tx_worker.clone();
            let rx = rx_worker.clone();
            let left_builder = self.left_builder.clone();
            let right_builder = self.right_builder.clone();

            tokio::spawn(async move {
                let ctx = Arc::new(SyncBackends {
                    left: left_builder.build().await?,
                    // left_cached: self.left_cached_builder.build().await?,
                    right: right_builder.build().await?,
                    // right_cached: self.right_cached_builder.build().await?,
                });

                // FIXME: lock occurs for too long
                while let Some(task) = rx.lock().await.recv().await {
                    println!("Worker {id} got a job; executing.");
                    tx.send(task(ctx.clone()).await).await.unwrap();
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }

                println!("no more task for {id}, exitting");

                Result::Ok(())
            });
        }

        Ok(ThreadPool { tx, rx })
    }
}

struct ThreadPool<L: BackendContext, R: BackendContext, T> {
    tx: mpsc::Sender<Task<L, R, T>>,
    rx: mpsc::Receiver<T>,
}

impl<L: BackendContext, R: BackendContext, T: Send + 'static> ThreadPool<L, R, T> {
    pub async fn send<F>(
        &mut self,
        task: impl FnOnce(Arc<SyncBackends<L, R>>) -> F + Send + Sync + 'static,
    ) where
        F: Future<Output = T> + Send + 'static,
    {
        let task: Task<L, R, T> = Box::new(move |ctx| Box::pin(task(ctx)));
        self.tx.send(task).await.unwrap();
    }

    pub async fn recv(&mut self) -> Option<T> {
        self.rx.recv().await
    }
}

pub type SyncEventHandler =
    dyn Fn(SyncEvent) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync;

/// The backend synchronization progress event.
///
/// Represents all the events that can be triggered during the backend
/// synchronization process.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum SyncEvent {
    ListedLeftFolders(usize),
    ListedRightFolders(usize),
}

impl SyncEvent {
    pub async fn emit(&self, handler: &Option<Arc<SyncEventHandler>>) {
        debug!("emitting sync event {self:?}");

        if let Some(handler) = handler.as_ref() {
            if let Err(err) = handler(self.clone()).await {
                debug!("error while emitting sync event: {err:?}");
            }
        }
    }
}

impl fmt::Display for SyncEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncEvent::ListedLeftFolders(n) => {
                write!(f, "Listed {n} left folders")
            }
            SyncEvent::ListedRightFolders(n) => {
                write!(f, "Listed {n} right folders")
            }
        }
    }
}

pub struct SyncBackends<L: BackendContext, R: BackendContext> {
    pub left: Backend<L>,
    // pub left_cached: Backend<MaildirContextSync>,
    pub right: Backend<R>,
    // pub right_cached: Backend<MaildirContextSync>,
}
