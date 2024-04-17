#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use super::{Flag, Flags};
use crate::{envelope::Id, AnyResult};

#[async_trait]
pub trait RemoveFlags: Send + Sync {
    /// Remove the given flags from envelope(s) matching the given id
    /// from the given folder.
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()>;

    /// Remove the given flag from envelope(s) matching the given id
    /// from the given folder.
    async fn remove_flag(&self, folder: &str, id: &Id, flag: Flag) -> AnyResult<()> {
        self.remove_flags(folder, id, &Flags::from_iter([flag]))
            .await
    }
}
