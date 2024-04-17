pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use super::Folders;
use crate::AnyResult;

#[async_trait]
pub trait ListFolders: Send + Sync {
    /// List all available folders (alias mailboxes).
    async fn list_folders(&self) -> AnyResult<Folders>;
}
