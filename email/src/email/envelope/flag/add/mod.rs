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
pub trait AddFlags: Send + Sync {
    /// Add the given flags to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()>;

    /// Add the given flag to envelope(s) matching the given id from
    /// the given folder.
    async fn add_flag(&self, folder: &str, id: &Id, flag: Flag) -> AnyResult<()> {
        self.add_flags(folder, id, &Flags::from_iter([flag])).await
    }
}
