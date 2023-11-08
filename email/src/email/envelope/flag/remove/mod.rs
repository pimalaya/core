use async_trait::async_trait;

use crate::{email::envelope::Id, Result};

use super::{Flag, Flags};

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait RemoveFlags: Send + Sync {
    /// Remove the given flags from envelope(s) matching the given id
    /// from the given folder.
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()>;

    /// Remove the given flag from envelope(s) matching the given id
    /// from the given folder.
    async fn remove_flag(&self, folder: &str, id: &Id, flag: Flag) -> Result<()> {
        self.remove_flags(folder, id, &Flags::from_iter([flag]))
            .await
    }
}
