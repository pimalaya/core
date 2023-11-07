//! Rust library to manage emails.
//!
//! The core concept of this library is to implement email actions and
//! to expose them into backend-agnostic abstractions. This way, you
//! can easily build email interfaces without caring about how to
//! connect to an IMAP server or how to send an email via SMTP.
//!
//! ## Backend features
//!
//! ### Folder
//!
//! - [`AddFolder`](crate::folder::AddFolder)
//! - [`ListFolders`](crate::folder::ListFolders)
//! - [`ExpungeFolder`](crate::folder::ExpungeFolder)
//! - [`PurgeFolder`](crate::folder::PurgeFolder)
//! - [`DeleteFolder`](crate::folder::DeleteFolder)
//!
//! ### Envelope
//!
//! - [`GetEnvelope`](crate::email::envelope::GetEnvelope)
//!
//! ### Flag
//!
//! - [`AddFlags`](crate::email::envelope::flag::AddFlags)
//!
//! ### Message
//!
//! - [`AddRawMessage`](crate::email::message::AddRawMessage)
//! - [`AddRawMessageWithFlags`](crate::email::message::AddRawMessageWithFlags) (implemented for `T: AddRawMessage + AddFlags`)
//! - [`PeekMessages`](crate::email::message::PeekMessages)
//!

pub mod account;
pub mod backend;
pub mod email;
pub mod folder;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod sender;

#[doc(inline)]
pub use backend::Backend;
#[doc(inline)]
pub use sender::Sender;

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
    #[cfg(feature = "pgp-native")]
    #[error(transparent)]
    NativePgpConfigError(#[from] account::config::pgp::native::Error),

    #[error(transparent)]
    MessageError(#[from] email::message::Error),
    #[error(transparent)]
    TplError(#[from] email::message::template::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapEnvelopeError(#[from] email::envelope::imap::Error),
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
    ImapBackendError(#[from] backend::imap::Error),
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

    #[cfg(feature = "imap-backend")]
    #[error("cannot list imap folders")]
    ListImapFoldersError(#[source] imap::Error),
    #[cfg(feature = "imap-backend")]
    #[error(transparent)]
    ImapError(#[from] imap::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
