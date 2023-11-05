use async_trait::async_trait;

use crate::Result;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddFolder: Send + Sync {
    /// Create the given folder.
    async fn add_folder(&self, folder: &str) -> Result<()>;
}
