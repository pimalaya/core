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
pub trait AddFlags: Send + Sync {
    /// Add the given flags to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> crate::Result<()>;

    /// Add the given flag to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> crate::Result<()> {
        self.add_flags(folder, id, &Flags::from_iter([flag])).await
    }
}
