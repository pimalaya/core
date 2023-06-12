pub mod backend;
pub mod domain;
pub mod sender;

pub use backend::{
    Backend, BackendBuilder, BackendConfig, BackendSyncBuilder, BackendSyncProgress,
    BackendSyncProgressEvent, BackendSyncReport, MaildirBackend, MaildirBackendBuilder,
    MaildirConfig,
};
#[cfg(feature = "imap-backend")]
pub use backend::{ImapAuthConfig, ImapBackend, ImapConfig};
#[cfg(feature = "notmuch-backend")]
pub use backend::{NotmuchBackend, NotmuchBackendBuilder, NotmuchConfig};
pub use domain::*;
pub use mail_builder::MessageBuilder as EmailBuilder;
pub use pimalaya_email_tpl::{FilterParts, ShowHeadersStrategy, Tpl, TplInterpreter};
pub use sender::{Sender, SenderBuilder, SenderConfig, Sendmail, SendmailConfig};
#[cfg(feature = "smtp-sender")]
pub use sender::{Smtp, SmtpAuthConfig, SmtpConfig};
