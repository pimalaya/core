//! Module dedicated to backend configuration.
//!
//! This module contains the backend configuration used for the
//! current account. One account can have only one backend and so one
//! backend configuration.

use std::ops::Deref;

#[cfg(feature = "imap-backend")]
use crate::imap::ImapConfig;
use crate::maildir::MaildirConfig;
#[cfg(feature = "notmuch-backend")]
use crate::notmuch::NotmuchConfig;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendConfig {
    pub default: bool,
    pub kind: BackendConfigKind,
}

impl Deref for BackendConfig {
    type Target = BackendConfigKind;

    fn deref(&self) -> &Self::Target {
        &self.kind
    }
}

/// The backend configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendConfigKind {
    /// The Maildir backend configuration.
    Maildir(MaildirConfig),

    /// The IMAP backend configuration.
    #[cfg(feature = "imap-backend")]
    Imap(ImapConfig),

    /// The notmuch backend configuration.
    #[cfg(feature = "notmuch-backend")]
    Notmuch(NotmuchConfig),
}
