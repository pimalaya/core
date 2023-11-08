use async_trait::async_trait;

use crate::Result;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait ExpungeFolder: Send + Sync {
    /// Expunge the given folder.
    ///
    /// The concept is similar to the IMAP expunge: it definitely
    /// deletes messages with [`Flag::Deleted`](crate::email::Flag).
    async fn expunge_folder(&self, folder: &str) -> Result<()>;
}
