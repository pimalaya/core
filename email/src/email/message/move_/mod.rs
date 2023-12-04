#[cfg(feature = "imap")]
pub mod imap;
pub mod maildir;

use async_trait::async_trait;

use crate::{envelope::Id, Result};

#[async_trait]
pub trait MoveMessages: Send + Sync {
    /// Move emails from the given folder to the given folder matching
    /// the given id.
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()>;
}
