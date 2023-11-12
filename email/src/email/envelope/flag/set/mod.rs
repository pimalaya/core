use async_trait::async_trait;

use crate::{email::envelope::Id, Result};

use super::{Flag, Flags};

#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;

#[async_trait]
pub trait SetFlags: Send + Sync {
    /// Set the given flags to envelope(s) matching the given id from
    /// the given folder.
    ///
    /// This function replaces any exsting flags by the given ones.
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()>;

    /// Set the given flag to envelope(s) matching the given id from
    /// the given folder.
    ///
    /// This function replaces any exsting flags by the given one.
    async fn set_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.set_flags(folder, id, &Flags::from_iter([flag])).await
    }
}
