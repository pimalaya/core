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
    folder::{
        sync::{patch::build_patch, FolderSyncHunk},
        Folder,
    },
    maildir::{config::MaildirConfig, MaildirContextBuilder, MaildirContextSync},
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

        let mut report = SyncReport::default();

        let cache_dir = self.get_cache_dir()?;
        let left_config = self.left_builder.account_config.clone();
        let right_config = self.left_builder.account_config.clone();

        let root_dir = cache_dir.join(&left_config.name);
        let ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig { root_dir }));
        let left_cache_builder = BackendBuilder::new(left_config.clone(), ctx);

        let root_dir = cache_dir.join(&right_config.name);
        let ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig { root_dir }));
        let right_cache_builder = BackendBuilder::new(right_config.clone(), ctx);

        let mut pool = ThreadPoolBuilder::new(
            left_cache_builder,
            self.left_builder.clone(),
            right_cache_builder,
            self.right_builder.clone(),
            self.handler.clone(),
            8,
        )
        .build()
        .await?;

        pool.send(|ctx| async move {
            let folders = ctx.left_cache.list_folders().await?;
            let names = HashSet::<String>::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedLeftCachedFolders(names.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(SyncTask::ListLeftCachedFolders(names))
        })
        .await;

        pool.send(|ctx| async move {
            let folders = ctx.left.list_folders().await?;
            let names = HashSet::<String>::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedLeftFolders(names.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(SyncTask::ListLeftFolders(names))
        })
        .await;

        pool.send(|ctx| async move {
            let folders = ctx.right_cache.list_folders().await?;
            let names = HashSet::<String>::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedRightCachedFolders(names.len())
                .emit(&ctx.handler)
                .await;

            Ok(SyncTask::ListRightCachedFolders(names))
        })
        .await;

        pool.send(|ctx| async move {
            let folders = ctx.right.list_folders().await?;
            let names = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedRightFolders(names.len())
                .emit(&ctx.handler)
                .await;

            Ok(SyncTask::ListRightFolders(names))
        })
        .await;

        let mut left_cached_folders = None::<HashSet<String>>;
        let mut left_folders = None::<HashSet<String>>;
        let mut right_cached_folders = None::<HashSet<String>>;
        let mut right_folders = None::<HashSet<String>>;

        loop {
            match pool.recv().await {
                None => break,
                Some(Err(err)) => Err(err)?,
                Some(Ok(SyncTask::ListLeftCachedFolders(names))) => {
                    left_cached_folders = Some(names);
                }
                Some(Ok(SyncTask::ListLeftFolders(names))) => {
                    left_folders = Some(names);
                }
                Some(Ok(SyncTask::ListRightCachedFolders(names))) => {
                    right_cached_folders = Some(names);
                }
                Some(Ok(SyncTask::ListRightFolders(names))) => {
                    right_folders = Some(names);
                }
                Some(Ok(_)) => {
                    // should not happen
                }
            }

            let ready = left_cached_folders.is_some()
                && left_folders.is_some()
                && right_cached_folders.is_some()
                && right_folders.is_some();

            if ready {
                break;
            }
        }

        let patch = build_patch(
            left_cached_folders.unwrap(),
            left_folders.unwrap(),
            right_cached_folders.unwrap(),
            right_folders.unwrap(),
        );

        let (folders, patch) = patch.into_iter().fold(
            (HashSet::default(), vec![]),
            |(mut folders, mut patch), (folder, hunks)| {
                folders.insert(folder);
                patch.extend(hunks);
                (folders, patch)
            },
        );

        report.folder.folders = folders;

        let mut patch_len = patch.len();

        for hunk in patch {
            match hunk.clone() {
                FolderSyncHunk::Cache(folder, SyncDestination::Left) => {
                    pool.send(|ctx| async move {
                        match ctx.left_cache.add_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Create(folder, SyncDestination::Left) => {
                    pool.send(|ctx| async move {
                        match ctx.left.add_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Cache(folder, SyncDestination::Right) => {
                    pool.send(|ctx| async move {
                        match ctx.right_cache.add_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Create(folder, SyncDestination::Right) => {
                    pool.send(|ctx| async move {
                        match ctx.right.add_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Uncache(folder, SyncDestination::Left) => {
                    pool.send(|ctx| async move {
                        match ctx.left_cache.delete_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Delete(folder, SyncDestination::Left) => {
                    pool.send(|ctx| async move {
                        match ctx.left.delete_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Uncache(folder, SyncDestination::Right) => {
                    pool.send(|ctx| async move {
                        match ctx.right_cache.delete_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
                FolderSyncHunk::Delete(folder, SyncDestination::Right) => {
                    pool.send(|ctx| async move {
                        match ctx.right.delete_folder(&folder).await {
                            Ok(()) => Ok(SyncTask::ProcessFolderHunk((hunk, None))),
                            Err(err) => Ok(SyncTask::ProcessFolderHunk((hunk, Some(err)))),
                        }
                    })
                    .await
                }
            }
        }

        loop {
            match pool.recv().await {
                None => break,
                Some(Err(err)) => Err(err)?,
                Some(Ok(SyncTask::ProcessFolderHunk(hunk))) => {
                    report.folder.patch.push(hunk);
                    patch_len -= 1;
                }
                Some(Ok(_)) => {
                    // should not happen
                }
            }

            if patch_len == 0 {
                break;
            }
        }

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
    dyn FnOnce(Arc<SyncPoolContext<L, R>>) -> Pin<Box<dyn Future<Output = T> + Send>> + Send + Sync,
>;

#[derive(Clone)]
struct ThreadPoolBuilder<L: BackendContextBuilder, R: BackendContextBuilder> {
    left_cache_builder: BackendBuilder<MaildirContextBuilder>,
    left_builder: BackendBuilder<L>,
    right_cache_builder: BackendBuilder<MaildirContextBuilder>,
    right_builder: BackendBuilder<R>,
    handler: Option<Arc<SyncEventHandler>>,
    size: usize,
}

impl<L: BackendContextBuilder + 'static, R: BackendContextBuilder + 'static>
    ThreadPoolBuilder<L, R>
{
    pub fn new(
        left_cache_builder: BackendBuilder<MaildirContextBuilder>,
        left_builder: BackendBuilder<L>,
        right_cache_builder: BackendBuilder<MaildirContextBuilder>,
        right_builder: BackendBuilder<R>,
        handler: Option<Arc<SyncEventHandler>>,
        size: usize,
    ) -> Self {
        Self {
            left_cache_builder,
            left_builder,
            right_cache_builder,
            right_builder,
            handler,
            size,
        }
    }

    pub async fn build<T: Send + 'static>(self) -> Result<ThreadPool<L::Context, R::Context, T>> {
        // channel for workers to receive and process tasks from the pool
        let (tx_pool, rx_worker) = mpsc::channel::<Task<L::Context, R::Context, T>>(1);
        let rx_workers = Arc::new(Mutex::new(rx_worker));

        // channel for workers to send output of their work to the pool
        let (tx_worker, rx_pool) = mpsc::channel::<T>(1);

        for id in 1..(self.size + 1) {
            println!("worker {id} init");
            let tx = tx_worker.clone();
            let rx = rx_workers.clone();
            let left_cache_builder = self.left_cache_builder.clone();
            let left_builder = self.left_builder.clone();
            let right_cache_builder = self.right_cache_builder.clone();
            let right_builder = self.right_builder.clone();
            let handler = self.handler.clone();

            tokio::spawn(async move {
                let (left_cache, left, right_cache, right) = tokio::try_join!(
                    left_cache_builder.build(),
                    left_builder.build(),
                    right_cache_builder.build(),
                    right_builder.build(),
                )?;

                let ctx = Arc::new(SyncPoolContext {
                    left_cache,
                    left,
                    right_cache,
                    right,
                    handler,
                });

                // FIXME: lock occurs for too long
                loop {
                    let mut lock = rx.lock().await;
                    match lock.recv().await {
                        None => break,
                        Some(task) => {
                            drop(lock);
                            println!("Worker {id} got a job; executing.");
                            tx.send(task(ctx.clone()).await).await.unwrap();
                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        }
                    }
                }

                println!("no more task for {id}, exitting");

                Result::Ok(())
            });
        }

        Ok(ThreadPool {
            tx: tx_pool,
            rx: rx_pool,
        })
    }
}

struct ThreadPool<L: BackendContext, R: BackendContext, T> {
    tx: mpsc::Sender<Task<L, R, T>>,
    rx: mpsc::Receiver<T>,
}

impl<L: BackendContext, R: BackendContext, T: Send + 'static> ThreadPool<L, R, T> {
    pub async fn send<F>(
        &mut self,
        task: impl FnOnce(Arc<SyncPoolContext<L, R>>) -> F + Send + Sync + 'static,
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
    ListedLeftCachedFolders(usize),
    ListedLeftFolders(usize),
    ListedRightCachedFolders(usize),
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
        }
    }
}

pub struct SyncPoolContext<L: BackendContext, R: BackendContext> {
    pub left_cache: Backend<MaildirContextSync>,
    pub left: Backend<L>,
    pub right_cache: Backend<MaildirContextSync>,
    pub right: Backend<R>,
    pub handler: Option<Arc<SyncEventHandler>>,
}

pub enum SyncTask {
    ListLeftCachedFolders(HashSet<String>),
    ListLeftFolders(HashSet<String>),
    ListRightCachedFolders(HashSet<String>),
    ListRightFolders(HashSet<String>),
    ProcessFolderHunk((FolderSyncHunk, Option<crate::Error>)),
}
