//! Backend config module.
//!
//! This module contains the representation of the backend
//! configuration of the user account.

#[cfg(feature = "imap-backend")]
use crate::ImapConfig;
use crate::MaildirConfig;
#[cfg(feature = "notmuch-backend")]
use crate::NotmuchConfig;

/// Represents the backend configuration of the user account.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackendConfig {
    #[default]
    None,
    Maildir(MaildirConfig),
    #[cfg(feature = "imap-backend")]
    Imap(ImapConfig),
    #[cfg(feature = "notmuch-backend")]
    Notmuch(NotmuchConfig),
}
