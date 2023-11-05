use async_trait::async_trait;

use crate::Result;

use super::Flags;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddEmail: Send + Sync {
    /// Add the given raw email with the given flags to the given
    /// folder.
    async fn add_email(&self, folder: &str, email: &[u8], flags: &Flags) -> Result<String>;
}
