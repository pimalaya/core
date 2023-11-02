use async_trait::async_trait;
use std::fmt::Debug;

use crate::Result;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait ExpungeFolder: Debug {
    /// Expunge the given folder.
    ///
    /// The concept is similar to the IMAP expunge: it definitely
    /// deletes emails that have the Deleted flag.
    async fn expunge_folder(&self, folder: &str) -> Result<()>;
}
