pub mod cache;
mod hunk;
mod patch;
mod report;
mod runner;

pub use self::cache::EnvelopeSyncCache;
pub use self::hunk::{EnvelopeSyncCacheHunk, EnvelopeSyncHunk};
pub use self::patch::{EnvelopeSyncCachePatch, EnvelopeSyncPatch, EnvelopeSyncPatchManager};
pub use self::report::EnvelopeSyncReport;
pub use self::runner::EnvelopeSyncRunner;
use crate::{
    account,
    backend::{self, sync::Destination},
    email, flag, AccountConfig, Backend, BackendBuilder, Envelope, MaildirBackendBuilder,
};

#[derive(Debug, thiserror::Error)]
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
    EmailError(#[from] email::Error),
    #[error(transparent)]
    BackendError(#[from] backend::Error),
    #[error(transparent)]
    MaildirBackendError(#[from] backend::maildir::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
