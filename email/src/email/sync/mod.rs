mod cache;
mod hunk;
mod patch;
mod report;
mod runner;

use std::result;
use thiserror::Error;

use crate::{
    account,
    backend::{self, sync::Destination},
    message, AccountConfig, Backend, BackendBuilder, Envelope, MaildirBackendBuilder,
};

pub use self::{
    cache::EmailSyncCache,
    hunk::{EmailSyncCacheHunk, EmailSyncHunk},
    patch::{EmailSyncCachePatch, EmailSyncPatch, EmailSyncPatchManager},
    report::EmailSyncReport,
    runner::EmailSyncRunner,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find email by internal id {0}")]
    FindEmailError(String),
    #[error("cannot find email by internal id {0}")]
    LockConnectionPoolCursorError(String),
    #[error("cannot find email by internal id {0}")]
    FindConnectionByCursorError(usize),
    #[error("cannot find email by internal id {0}")]
    LockConnectionError(String),

    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    EmailError(#[from] message::Error),
    #[error(transparent)]
    BackendError(#[from] backend::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] backend::maildir::Error),
}

pub type Result<T> = result::Result<T, Error>;
