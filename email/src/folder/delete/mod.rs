#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;

use crate::AnyResult;

#[async_trait]
pub trait DeleteFolder: Send + Sync {
    /// Definitely delete the given folder.
    ///
    /// Manipulate with caution: all emails contained in the given
    /// folder are also definitely deleted.
    async fn delete_folder(&self, folder: &str) -> AnyResult<()>;
}
