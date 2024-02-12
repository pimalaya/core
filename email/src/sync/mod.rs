pub mod pool;
pub mod report;

use advisory_lock::{AdvisoryFileLock, FileLockError, FileLockMode};
use futures::{stream::FuturesUnordered, Future, StreamExt};
use log::debug;
use std::{
    collections::{HashMap, HashSet},
    env, fmt,
    fs::OpenOptions,
    io,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
};
use thiserror::Error;

use crate::{
    backend::{BackendBuilder, BackendContextBuilder},
    email::{self, sync::EmailSyncHunk},
    envelope::{Envelope, Id},
    flag::Flag,
    folder::{self, sync::FolderSyncHunk, Folder},
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
    #[error("cannot find message associated to envelope {0}")]
    FindMessageError(String),
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

    pub fn get_left_cache_builder(&self) -> Result<BackendBuilder<MaildirContextBuilder>> {
        let left_config = self.left_builder.account_config.clone();
        let root_dir = self.get_cache_dir()?.join(&left_config.name);
        let ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig { root_dir }));
        let left_cache_builder = BackendBuilder::new(left_config.clone(), ctx);
        Ok(left_cache_builder)
    }

    pub fn get_right_cache_builder(&self) -> Result<BackendBuilder<MaildirContextBuilder>> {
        let right_config = self.right_builder.account_config.clone();
        let root_dir = self.get_cache_dir()?.join(&right_config.name);
        let ctx = MaildirContextBuilder::new(Arc::new(MaildirConfig { root_dir }));
        let right_cache_builder = BackendBuilder::new(right_config.clone(), ctx);
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

        let mut report = SyncReport::default();

        let pool = pool::new(
            self.get_left_cache_builder()?,
            self.left_builder.clone(),
            self.get_right_cache_builder()?,
            self.right_builder.clone(),
            self.handler.clone(),
        )
        .await?;

        let left_cached_folders = pool.exec(|ctx| async move {
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

            Result::Ok(names)
        });

        let left_folders = pool.exec(|ctx| async move {
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

            Result::Ok(names)
        });

        let right_cached_folders = pool.exec(|ctx| async move {
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

            Result::Ok(names)
        });

        let right_folders = pool.exec(|ctx| async move {
            let folders = ctx.right.list_folders().await?;
            let names: HashSet<String> = HashSet::from_iter(
                folders
                    .iter()
                    .map(Folder::get_kind_or_name)
                    .map(ToOwned::to_owned),
            );

            SyncEvent::ListedRightFolders(names.len())
                .emit(&ctx.handler)
                .await;

            Result::Ok(names)
        });

        let (left_cached_folders, left_folders, right_cached_folders, right_folders) = tokio::try_join!(
            left_cached_folders,
            left_folders,
            right_cached_folders,
            right_folders
        )?;

        SyncEvent::ListedAllFolders.emit(&self.handler).await;

        let patch = folder::sync::patch::build_patch(
            left_cached_folders,
            left_folders,
            right_cached_folders,
            right_folders,
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
        report.folder.patch = FuturesUnordered::from_iter(patch.into_iter().map(|hunk| {
            pool.exec(move |ctx| {
                let hunk_clone = hunk.clone();
                let task = async move {
                    match hunk_clone {
                        FolderSyncHunk::Cache(folder, SyncDestination::Left) => {
                            ctx.left_cache.add_folder(&folder).await
                        }
                        FolderSyncHunk::Create(folder, SyncDestination::Left) => {
                            ctx.left.add_folder(&folder).await
                        }
                        FolderSyncHunk::Cache(folder, SyncDestination::Right) => {
                            ctx.right_cache.add_folder(&folder).await
                        }
                        FolderSyncHunk::Create(folder, SyncDestination::Right) => {
                            ctx.right.add_folder(&folder).await
                        }
                        FolderSyncHunk::Uncache(folder, SyncDestination::Left) => {
                            ctx.left_cache.delete_folder(&folder).await
                        }
                        FolderSyncHunk::Delete(folder, SyncDestination::Left) => {
                            ctx.left.delete_folder(&folder).await
                        }
                        FolderSyncHunk::Uncache(folder, SyncDestination::Right) => {
                            ctx.right_cache.delete_folder(&folder).await
                        }
                        FolderSyncHunk::Delete(folder, SyncDestination::Right) => {
                            ctx.right.delete_folder(&folder).await
                        }
                    }
                };

                async move {
                    match task.await {
                        Ok(()) => (hunk, None),
                        Err(err) => (hunk, Some(err)),
                    }
                }
            })
        }))
        .collect::<Vec<_>>()
        .await;

        let patch = FuturesUnordered::from_iter(report.folder.folders.iter().map(|folder_ref| {
            let folder = folder_ref.clone();
            let left_cached_envelopes = pool.exec(|ctx| async move {
                let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                    ctx.left_cache
                        .list_envelopes(&folder, 0, 0)
                        .await?
                        .into_iter()
                        .map(|e| (e.message_id.clone(), e)),
                );

                SyncEvent::ListedLeftCachedEnvelopes(folder.clone(), envelopes.len())
                    .emit(&ctx.handler)
                    .await;

                Result::Ok(envelopes)
            });

            let folder = folder_ref.clone();
            let left_envelopes = pool.exec(|ctx| async move {
                let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                    ctx.left
                        .list_envelopes(&folder, 0, 0)
                        .await?
                        .into_iter()
                        .map(|e| (e.message_id.clone(), e)),
                );

                SyncEvent::ListedLeftEnvelopes(folder.clone(), envelopes.len())
                    .emit(&ctx.handler)
                    .await;

                Result::Ok(envelopes)
            });

            let folder = folder_ref.clone();
            let right_cached_envelopes = pool.exec(|ctx| async move {
                let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                    ctx.right_cache
                        .list_envelopes(&folder, 0, 0)
                        .await?
                        .into_iter()
                        .map(|e| (e.message_id.clone(), e)),
                );

                SyncEvent::ListedRightCachedEnvelopes(folder.clone(), envelopes.len())
                    .emit(&ctx.handler)
                    .await;

                Result::Ok(envelopes)
            });

            let folder = folder_ref.clone();
            let right_envelopes = pool.exec(|ctx| async move {
                let envelopes: HashMap<String, Envelope> = HashMap::from_iter(
                    ctx.right
                        .list_envelopes(&folder, 0, 0)
                        .await?
                        .into_iter()
                        .map(|e| (e.message_id.clone(), e)),
                );

                SyncEvent::ListedRightEnvelopes(folder.clone(), envelopes.len())
                    .emit(&ctx.handler)
                    .await;

                Result::Ok(envelopes)
            });

            async move {
                let (
                    left_cached_envelopes,
                    left_envelopes,
                    right_cached_envelopes,
                    right_envelopes,
                ) = tokio::try_join!(
                    left_cached_envelopes,
                    left_envelopes,
                    right_cached_envelopes,
                    right_envelopes
                )?;

                Result::Ok(email::sync::patch::build_patch(
                    folder_ref,
                    left_cached_envelopes,
                    left_envelopes,
                    right_cached_envelopes,
                    right_envelopes,
                ))
            }
        }))
        .filter_map(|res| async {
            match res {
                Ok(res) => Some(res),
                Err(err) => {
                    debug!("cannot join tasks: {err}");
                    None
                }
            }
        })
        .fold(Vec::new(), |mut patch, p| async {
            patch.extend(p.into_iter().flatten());
            patch
        })
        .await;

        report.email.patch = FuturesUnordered::from_iter(patch.into_iter().map(|hunk| {
            pool.exec(move |ctx| {
                let hunk_clone = hunk.clone();
                let task = async move {
                    match hunk_clone {
                        EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Left) => {
                            let envelope = ctx.left.get_envelope(&folder, &Id::single(id)).await?;
                            let flags = envelope.flags.clone();
                            let msg = Vec::<u8>::try_from(envelope)?;
                            ctx.left_cache
                                .add_message_with_flags(&folder, &msg, &flags)
                                .await?;
                            Ok(())
                        }
                        EmailSyncHunk::GetThenCache(folder, id, SyncDestination::Right) => {
                            let envelope = ctx.right.get_envelope(&folder, &Id::single(id)).await?;
                            let flags = envelope.flags.clone();
                            let msg = Vec::<u8>::try_from(envelope)?;
                            ctx.right_cache
                                .add_message_with_flags(&folder, &msg, &flags)
                                .await?;
                            Ok(())
                        }
                        EmailSyncHunk::CopyThenCache(
                            folder,
                            envelope,
                            source,
                            target,
                            refresh_source_cache,
                        ) => {
                            let id = Id::single(&envelope.id);
                            let msgs = match source {
                                SyncDestination::Left => {
                                    if refresh_source_cache {
                                        let flags = envelope.flags.clone();
                                        let msg = Vec::<u8>::try_from(envelope.clone())?;
                                        ctx.left_cache
                                            .add_message_with_flags(&folder, &msg, &flags)
                                            .await?;
                                    };
                                    ctx.left.peek_messages(&folder, &id).await?
                                }
                                SyncDestination::Right => {
                                    if refresh_source_cache {
                                        let flags = envelope.flags.clone();
                                        let msg = Vec::<u8>::try_from(envelope.clone())?;
                                        ctx.right_cache
                                            .add_message_with_flags(&folder, &msg, &flags)
                                            .await?;
                                    };
                                    ctx.right.peek_messages(&folder, &id).await?
                                }
                            };

                            let msgs = msgs.to_vec();
                            let msg = msgs
                                .first()
                                .ok_or_else(|| Error::FindMessageError(envelope.id.clone()))?;

                            match target {
                                SyncDestination::Left => {
                                    let id = ctx
                                        .left
                                        .add_message_with_flags(
                                            &folder,
                                            msg.raw()?,
                                            &envelope.flags,
                                        )
                                        .await?;
                                    let envelope =
                                        ctx.left.get_envelope(&folder, &Id::single(id)).await?;
                                    let flags = envelope.flags.clone();
                                    let msg = Vec::<u8>::try_from(envelope)?;
                                    ctx.left_cache
                                        .add_message_with_flags(&folder, &msg, &flags)
                                        .await?;
                                    Ok(())
                                }
                                SyncDestination::Right => {
                                    let id = ctx
                                        .right
                                        .add_message_with_flags(
                                            &folder,
                                            msg.raw()?,
                                            &envelope.flags,
                                        )
                                        .await?;
                                    let envelope =
                                        ctx.right.get_envelope(&folder, &Id::single(id)).await?;
                                    let flags = envelope.flags.clone();
                                    let msg = Vec::<u8>::try_from(envelope)?;
                                    ctx.right_cache
                                        .add_message_with_flags(&folder, &msg, &flags)
                                        .await?;
                                    Ok(())
                                }
                            }
                        }
                        EmailSyncHunk::Uncache(folder, id, SyncDestination::Left) => {
                            ctx.left_cache
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::Delete(folder, id, SyncDestination::Left) => {
                            ctx.left
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::Uncache(folder, id, SyncDestination::Right) => {
                            ctx.right_cache
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::Delete(folder, id, SyncDestination::Right) => {
                            ctx.right
                                .add_flag(&folder, &Id::single(id), Flag::Deleted)
                                .await
                        }
                        EmailSyncHunk::UpdateCachedFlags(
                            folder,
                            envelope,
                            SyncDestination::Left,
                        ) => {
                            ctx.left_cache
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                        EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Left) => {
                            ctx.left
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                        EmailSyncHunk::UpdateCachedFlags(
                            folder,
                            envelope,
                            SyncDestination::Right,
                        ) => {
                            ctx.right_cache
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                        EmailSyncHunk::UpdateFlags(folder, envelope, SyncDestination::Right) => {
                            ctx.right
                                .set_flags(&folder, &Id::single(&envelope.id), &envelope.flags)
                                .await
                        }
                    }
                };

                async move {
                    match task.await {
                        Ok(()) => (hunk, None),
                        Err(err) => (hunk, Some(err)),
                    }
                }
            })
        }))
        .collect::<Vec<_>>()
        .await;

        FuturesUnordered::from_iter(report.folder.folders.iter().map(|folder_ref| {
            let folder = folder_ref.clone();
            let left_cached_expunge =
                pool.exec(|ctx| async move { ctx.left_cache.expunge_folder(&folder).await });

            let folder = folder_ref.clone();
            let left_expunge =
                pool.exec(|ctx| async move { ctx.left.expunge_folder(&folder).await });

            let folder = folder_ref.clone();
            let right_cached_expunge =
                pool.exec(|ctx| async move { ctx.right_cache.expunge_folder(&folder).await });

            let folder = folder_ref.clone();
            let right_expunge =
                pool.exec(|ctx| async move { ctx.right.expunge_folder(&folder).await });

            async move {
                tokio::try_join!(
                    left_cached_expunge,
                    left_expunge,
                    right_cached_expunge,
                    right_expunge
                )
            }
        }))
        .filter_map(|res| async {
            match res {
                Ok(res) => Some(res),
                Err(err) => {
                    debug!("cannot join tasks: {err}");
                    None
                }
            }
        })
        .for_each(|_| async {})
        .await;

        pool.shutdown().await;

        debug!("unlocking sync file");
        lock_file
            .unlock()
            .map_err(|err| Error::UnlockFileError(err, lock_file_path))?;

        Ok(report)
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
    ListedAllFolders,
    ProcessedFolderHunk(FolderSyncHunk),
    ListedLeftCachedEnvelopes(String, usize),
    ListedLeftEnvelopes(String, usize),
    ListedRightCachedEnvelopes(String, usize),
    ListedRightEnvelopes(String, usize),
    ListedAllEnvelopes,
    ProcessedEmailHunk(EmailSyncHunk),
}

impl SyncEvent {
    pub async fn emit(&self, handler: &Option<Arc<SyncEventHandler>>) {
        if let Some(handler) = handler.as_ref() {
            if let Err(err) = handler(self.clone()).await {
                debug!("error while emitting sync event: {err:?}");
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
            SyncEvent::ProcessedFolderHunk(hunk) => {
                write!(f, "{hunk}")
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
            SyncEvent::ListedAllEnvelopes => {
                write!(f, "Listed all envelopes from all folders")
            }
            SyncEvent::ProcessedEmailHunk(hunk) => {
                write!(f, "{hunk}")
            }
        }
    }
}
