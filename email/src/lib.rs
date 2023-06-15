pub mod account;
pub mod backend;
pub mod email;
pub mod flag;
pub mod folder;
pub mod sender;

#[doc(inline)]
pub use account::{
    AccountConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig, DEFAULT_INBOX_FOLDER,
};
#[doc(inline)]
pub use backend::{
    Backend, BackendBuilder, BackendConfig, BackendSyncBuilder, BackendSyncProgress,
    BackendSyncProgressEvent, BackendSyncReport, MaildirBackend, MaildirBackendBuilder,
    MaildirConfig,
};
#[cfg(feature = "imap-backend")]
#[doc(inline)]
pub use backend::{ImapAuthConfig, ImapBackend, ImapConfig};
#[cfg(feature = "notmuch-backend")]
#[doc(inline)]
pub use backend::{NotmuchBackend, NotmuchBackendBuilder, NotmuchConfig};
#[doc(inline)]
pub use email::{
    envelope::{self, *},
    Attachment, Email, EmailHooks, EmailTextPlainFormat, Emails, ForwardTplBuilder, NewTplBuilder,
    ReplyTplBuilder,
};
#[doc(inline)]
pub use flag::{Flag, Flags};
#[doc(inline)]
pub use folder::{
    Folder, FolderSyncCache, FolderSyncCacheHunk, FolderSyncCachePatch, FolderSyncHunk,
    FolderSyncPatch, FolderSyncPatchManager, FolderSyncPatches, FolderSyncStrategy, Folders,
};
#[doc(inline)]
pub use sender::{Sender, SenderBuilder, SenderConfig, Sendmail, SendmailConfig};
#[cfg(feature = "smtp-sender")]
#[doc(inline)]
pub use sender::{Smtp, SmtpAuthConfig, SmtpConfig};

pub use mail_builder::MessageBuilder as EmailBuilder;
pub use pimalaya_email_tpl::{FilterParts, ShowHeadersStrategy, Tpl, TplInterpreter};
