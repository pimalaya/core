mod backend;
mod config;

#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;

pub use self::backend::{
    Backend, BackendBuilder, BackendSyncBuilder, BackendSyncProgressEvent, Error, Result,
};
pub use self::config::BackendConfig;
#[cfg(feature = "imap-backend")]
pub use self::imap::*;
pub use self::maildir::*;
#[cfg(feature = "notmuch-backend")]
pub use self::notmuch::{NotmuchBackend, NotmuchConfig};
