//! Module dedicated to backend configuration.
//!
//! This module contains the backend configuration used for the
//! current account. One account can have only one backend and so one
//! backend configuration.

#[cfg(feature = "imap-backend")]
use crate::backend::ImapConfig;
use crate::backend::MaildirConfig;
#[cfg(feature = "notmuch-backend")]
use crate::backend::NotmuchConfig;

/// The backend configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum BackendConfig {
    /// The undefined backend is useful when you need to create an
    /// account that only send emails using a [crate::Sender].
    #[default]
    None,

    /// The Maildir backend configuration.
    Maildir(MaildirConfig),

    /// The IMAP backend configuration.
    #[cfg(feature = "imap-backend")]
    Imap(ImapConfig),

    /// The notmuch backend configuration.
    #[cfg(feature = "notmuch-backend")]
    Notmuch(NotmuchConfig),
}
