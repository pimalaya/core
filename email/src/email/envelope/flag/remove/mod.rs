#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::envelope::Id;

use super::{Flag, Flags};

#[async_trait]
pub trait RemoveFlags: Send + Sync {
    /// Remove the given flags from envelope(s) matching the given id
    /// from the given folder.
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> crate::Result<()>;

    /// Remove the given flag from envelope(s) matching the given id
    /// from the given folder.
    async fn remove_flag(&self, folder: &str, id: &Id, flag: Flag) -> crate::Result<()> {
        self.remove_flags(folder, id, &Flags::from_iter([flag]))
            .await
    }
}
