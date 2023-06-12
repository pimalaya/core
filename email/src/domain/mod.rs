pub mod account;
pub mod email;
pub mod envelope;
pub mod flag;
pub mod folder;

pub use account::{
    AccountConfig, OAuth2Config, OAuth2Method, OAuth2Scopes, PasswdConfig, DEFAULT_INBOX_FOLDER,
};
pub use email::{
    Attachment, Email, EmailHooks, EmailTextPlainFormat, Emails, ForwardTplBuilder, NewTplBuilder,
    ReplyTplBuilder,
};
pub use envelope::{Envelope, EnvelopeSyncPatch, EnvelopeSyncPatchManager, Envelopes};
pub use flag::{Flag, Flags};
pub use folder::{Folder, FolderSyncPatch, FolderSyncPatchManager, Folders};
