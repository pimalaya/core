//! Module dedicated to email synchronization.
//!
//! The core concept of this module is the [`EmailSyncPatchManager`],
//! which allows you to synchronize remote emails using a local
//! Maildir backend.

mod cache;
mod hunk;
mod patch;
mod report;
mod runner;

use thiserror::Error;

use crate::{
    account::config::AccountConfig, backend::BackendBuilder, envelope::Envelope,
    sync::SyncDestination,
};

#[doc(inline)]
pub use self::{
    cache::EmailSyncCache,
    hunk::{EmailSyncCacheHunk, EmailSyncHunk},
    patch::{EmailSyncCachePatch, EmailSyncPatch, EmailSyncPatchManager},
    report::EmailSyncReport,
    runner::EmailSyncRunner,
};

/// Errors related to email synchronization.
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
