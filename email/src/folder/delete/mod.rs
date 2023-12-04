use async_trait::async_trait;

use crate::Result;

#[cfg(feature = "imap")]
pub mod imap;
pub mod maildir;

#[async_trait]
pub trait DeleteFolder: Send + Sync {
    /// Definitely delete the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are also definitely deleted.
    async fn delete_folder(&self, folder: &str) -> Result<()>;
}
