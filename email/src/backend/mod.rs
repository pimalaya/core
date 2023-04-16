mod backend;
mod config;
pub mod id_mapper;

#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;

pub use self::backend::{
    Backend, BackendBuilder, BackendSyncBuilder, BackendSyncProgressEvent, Error, Result,
};
pub use self::config::BackendConfig;
pub use self::id_mapper::IdMapper;
#[cfg(feature = "imap-backend")]
pub use self::imap::{ImapBackend, ImapBackendBuilder, ImapConfig};
pub use self::maildir::{MaildirBackend, MaildirConfig};
#[cfg(feature = "notmuch-backend")]
pub use self::notmuch::{NotmuchBackend, NotmuchConfig};
