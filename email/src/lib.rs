//! Rust library to manage your emails.
//!
//! The core concept of this library is to implement email actions and
//! to expose them into backend-agnostic abstractions. This way, you
//! can easily build email interfaces without caring about how to
//! connect to an IMAP server or how to send an email via SMTP.
//!
//! Here some key structures to better understand the concept of the
//! library:
//!
//!  - [`AccountConfig`](account::AccountConfig)
//!  - [`Folder`](folder::Folder)
//!  - [`Envelope`](email::Envelope)
//!  - [`Message`](email::Message)
//!  - [`Flag`](email::Flag)
//!  - [`Backend`](backend::Backend)
//!  - [`Sender`](sender::Sender)

pub mod account;
pub mod backend;
pub mod email;
pub mod folder;
pub mod sender;

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    AccountConfigError(#[from] account::config::Error),
    #[error(transparent)]
    OAuth2ConfigError(#[from] account::config::oauth2::Error),
    #[error(transparent)]
    PasswdConfigError(#[from] account::config::passwd::Error),
    #[error(transparent)]
    AccountSyncError(#[from] account::sync::Error),

    #[error(transparent)]
    MessageError(#[from] email::message::Error),
    #[error(transparent)]
    TplError(#[from] email::message::template::Error),
    #[error(transparent)]
    FlagError(#[from] email::envelope::flag::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapFlagError(#[from] email::envelope::flag::imap::Error),
    #[error(transparent)]
    MaildirFlagError(#[from] email::envelope::flag::maildir::Error),
    #[error(transparent)]
    EmailSyncError(#[from] email::sync::Error),

    #[error(transparent)]
    BackendError(#[from] backend::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapError(#[from] backend::imap::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapConfigError(#[from] backend::imap::config::Error),
    #[error(transparent)]
    MaildirError(#[from] backend::maildir::Error),
    #[cfg(feature = "notmuch-backend")]
    #[error(transparent)]
    NotmuchError(#[from] backend::notmuch::Error),

    #[error(transparent)]
    SenderError(#[from] sender::Error),
    #[error(transparent)]
    SendmailError(#[from] sender::sendmail::Error),
    #[cfg(feature = "smtp-sender")]
    #[error(transparent)]
    SmtpError(#[from] sender::smtp::Error),
    #[cfg(feature = "smtp-sender")]
    #[error(transparent)]
    SmtpConfigError(#[from] sender::smtp::config::Error),

    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) trait UnwrapOrWarn<T> {
    fn unwrap_or_warn(self, warn: &str) -> T;
}

impl<T: Default> UnwrapOrWarn<T> for std::option::Option<T> {
    fn unwrap_or_warn(self, warn: &str) -> T {
        match self {
            Some(t) => t,
            None => {
                log::warn!("{warn}");
                log::debug!("{warn}");
                Default::default()
            }
        }
    }
}

impl<T: Default, E: std::error::Error> UnwrapOrWarn<T> for std::result::Result<T, E> {
    fn unwrap_or_warn(self, warn: &str) -> T {
        match self {
            Ok(t) => t,
            Err(e) => {
                log::warn!("{warn}: {e}");
                log::debug!("{warn}: {e:?}");
                Default::default()
            }
        }
    }
}
