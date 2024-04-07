#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;

use crate::AnyResult;

#[async_trait]
pub trait ExpungeFolder: Send + Sync {
    /// Expunge the given folder.
    ///
    /// The concept is similar to the IMAP expunge: it definitely
    /// deletes messages with [`Flag::Deleted`](crate::email::Flag).
    async fn expunge_folder(&self, folder: &str) -> AnyResult<()>;
}
