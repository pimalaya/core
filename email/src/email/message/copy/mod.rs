#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::{envelope::Id, AnyResult};

#[async_trait]
pub trait CopyMessages: Send + Sync {
    /// Copy emails from the given folder to the given folder
    /// matching the given id.
    async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()>;
}
