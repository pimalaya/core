#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::{envelope::Id, AnyResult};

/// Feature to remove message(s).
#[async_trait]
pub trait RemoveMessages: Send + Sync {
    /// Remove messages from the given folder matching the given
    /// envelope id(s).
    ///
    /// This function definitely remove message(s). If you are looking
    /// for its soft version, see [`super::delete::DeleteMessages`].
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()>;
}
