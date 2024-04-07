#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::AnyResult;

#[async_trait]
pub trait AddFolder: Send + Sync {
    /// Create the given folder.
    async fn add_folder(&self, folder: &str) -> AnyResult<()>;
}
