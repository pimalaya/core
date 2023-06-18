mod cache;
mod hunk;
mod patch;
mod report;
mod runner;

use thiserror::Error;

use crate::{
    backend::sync::Destination, AccountConfig, Backend, BackendBuilder, Envelope,
    MaildirBackendBuilder,
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
}
