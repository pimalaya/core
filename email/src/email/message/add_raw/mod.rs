use async_trait::async_trait;

use crate::{envelope::SingleId, Result};

#[cfg(feature = "imap")]
pub mod imap;

#[async_trait]
pub trait AddRawMessage: Send + Sync {
    /// Add the given raw email message to the given folder.
    ///
    /// This function returns the identifier of the newly added
    /// message to the folder.
    async fn add_raw_message(&self, folder: &str, raw_msg: &[u8]) -> Result<SingleId>;
}
