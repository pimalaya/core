use async_trait::async_trait;

use crate::Result;

use super::{Folder, Folders};

#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;

#[async_trait]
pub trait ListFolders: Send + Sync {
    /// List all available folders (alias mailboxes).
    async fn list_folders(&self) -> Result<Folders>;
}
