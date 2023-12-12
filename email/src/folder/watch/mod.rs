pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait WatchFolder: Send + Sync {
    /// Watch the given folder and execute shell commands defined in [`crate::account::config::AccountConfig`] on
    /// change.
    async fn watch_folder(&self, folder: &str) -> Result<()>;
}
