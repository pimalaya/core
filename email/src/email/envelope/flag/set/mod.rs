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
pub trait SetFlags: Send + Sync {
    /// Set the given flags to envelope(s) matching the given id from
    /// the given folder.
    ///
    /// This function replaces any exsting flags by the given ones.
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> crate::Result<()>;

    /// Set the given flag to envelope(s) matching the given id from
    /// the given folder.
    ///
    /// This function replaces any exsting flags by the given one.
    async fn set_flag(&self, folder: &str, id: &Id, flag: Flag) -> crate::Result<()> {
        self.set_flags(folder, id, &Flags::from_iter([flag])).await
    }
}
