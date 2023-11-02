use async_trait::async_trait;
use std::fmt::Debug;

use crate::Result;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait PurgeFolder: Debug {
    /// Purge the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are definitely deleted.
    async fn purge_folder(&self, folder: &str) -> Result<()>;
}
