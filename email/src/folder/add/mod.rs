use async_trait::async_trait;
use std::fmt::Debug;

use crate::Result;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddFolder: Debug {
    /// Create the given folder.
    async fn add_folder(&self, folder: &str) -> Result<()>;
}
