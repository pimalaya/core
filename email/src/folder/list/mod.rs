pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;

use crate::Result;

use super::Folders;

#[async_trait]
pub trait ListFolders: Send + Sync {
    /// List all available folders (alias mailboxes).
    async fn list_folders(&self) -> Result<Folders>;
}
