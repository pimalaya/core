#[cfg(feature = "imap")]
pub mod imap;

use async_trait::async_trait;

use crate::AnyResult;

#[async_trait]
pub trait PurgeFolder: Send + Sync {
    /// Purge the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are definitely deleted.
    async fn purge_folder(&self, folder: &str) -> AnyResult<()>;
}
