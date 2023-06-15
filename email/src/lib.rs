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
    email::*,
    folder::{
        Folder, FolderSyncCache, FolderSyncCacheHunk, FolderSyncCachePatch, FolderSyncHunk,
        FolderSyncPatch, FolderSyncPatchManager, FolderSyncPatches, FolderSyncStrategy, Folders,
    },
    sender::{Sender, SenderBuilder, SenderConfig, Sendmail, SendmailConfig},
};

pub use mail_builder::MessageBuilder as EmailBuilder;
pub use pimalaya_email_tpl::{FilterParts, ShowHeadersStrategy, Tpl, TplInterpreter};
