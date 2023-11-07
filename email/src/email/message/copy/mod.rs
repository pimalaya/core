use async_trait::async_trait;

use crate::{email::envelope::Id, Result};

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait CopyMessages: Send + Sync {
    /// Copy emails from the given folder to the given folder
    /// matching the given id.
    async fn copy_emails(&self, from_folder: &str, to_folder: &str, id: Id) -> Result<()>;
}
