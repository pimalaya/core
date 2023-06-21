//! Rust library to manage your emails.
//!
//! The core concept of this library is to implement email actions and
//! to expose them into backend-agnostic abstractions. This way, you
//! can easily build email interfaces without caring about how to
//! connect to an IMAP server or how to send an email via SMTP.
//!
//! The [account] module exposes stuff related to account
//! management. The most important structure is [AccountConfig] which
//! contains all the configuration of the current account being
//! manipulated. Other modules heavily rely on.
//!
//! The [folder] module exposes stuff related to folder (or mailbox)
//! management.
//!
//! The [email] module exposes stuff related to email management,
//! which includes [envelope], [message], [flag], [template] etc.
//!
//! The [backend] module exposes stuff related to email
//! manipulation. The main structure is the [Backend] interface, which
//! abstracts how emails are manipulated. The library comes with few
//! implementations (IMAP, Maildir, Notmuch) but you can build your
//! own.
//!
//! The [sender] module exposes stuff related to email sending. The
//! main structure is the [Sender] interface, which abstracts how
//! emails are sent. The library comes with few implementations (SMTP,
//! Sendmail) but you can build your own.

pub mod account;
pub mod backend;
pub mod email;
pub mod folder;
pub mod sender;

#[cfg(feature = "imap-backend")]
#[doc(inline)]
pub use self::backend::{ImapAuthConfig, ImapBackend, ImapConfig};
#[cfg(feature = "notmuch-backend")]
#[doc(inline)]
pub use self::backend::{NotmuchBackend, NotmuchBackendBuilder, NotmuchConfig};
#[cfg(feature = "smtp-sender")]
#[doc(inline)]
pub use self::sender::{Smtp, SmtpAuthConfig, SmtpConfig};
#[doc(inline)]
pub use self::{
    account::{
        AccountConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig, DEFAULT_INBOX_FOLDER,
    },
    backend::{
        Backend, BackendBuilder, BackendConfig, BackendSyncBuilder, BackendSyncProgress,
        BackendSyncProgressEvent, BackendSyncReport, MaildirBackend, MaildirBackendBuilder,
        MaildirConfig,
    },
    email::{
        envelope, flag, message, template, Address, EmailHooks, EmailSyncCache, EmailSyncCacheHunk,
        EmailSyncCachePatch, EmailSyncHunk, EmailSyncPatch, EmailSyncPatchManager, EmailSyncReport,
        EmailTextPlainFormat, Envelope, Envelopes, Flag, Flags, Message, Messages,
    },
    folder::{
        Folder, FolderSyncCache, FolderSyncCacheHunk, FolderSyncCachePatch, FolderSyncHunk,
        FolderSyncPatch, FolderSyncPatchManager, FolderSyncPatches, FolderSyncStrategy, Folders,
    },
    sender::{Sender, SenderBuilder, SenderConfig, Sendmail, SendmailConfig},
};

pub use mail_builder::MessageBuilder as EmailBuilder;
pub use pimalaya_email_tpl::{FilterParts, ShowHeadersStrategy, Tpl, TplInterpreter};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    AccountConfigError(#[from] account::config::Error),
    #[error(transparent)]
    OAuth2ConfigError(#[from] account::config::oauth2::Error),
    #[error(transparent)]
    PasswdConfigError(#[from] account::config::passwd::Error),

    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    EmailSyncError(#[from] email::sync::Error),
    #[error(transparent)]
    TplError(#[from] email::message::template::Error),
    #[error(transparent)]
    FlagError(#[from] email::envelope::flag::Error),

    #[error(transparent)]
    BackendError(#[from] backend::Error),
    #[error(transparent)]
    BackendSyncError(#[from] backend::sync::Error),
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

pub type Result<T> = std::result::Result<T, Error>;
