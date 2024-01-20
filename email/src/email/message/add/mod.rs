pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::{
    envelope::SingleId,
    flag::{Flag, Flags},
    Result,
};

#[async_trait]
pub trait AddMessage: Send + Sync {
    /// Add the given raw email message with the given flags to the
    /// given folder.
    async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId>;

    /// Add the given raw email message with the given flag to the
    /// given folder.
    async fn add_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<SingleId> {
        self.add_message_with_flags(folder, raw_msg, &Flags::from_iter([flag]))
            .await
    }

    /// Add the given raw email message to the given folder.
    async fn add_message(&self, folder: &str, raw_msg: &[u8]) -> Result<SingleId> {
        self.add_message_with_flags(folder, raw_msg, &Default::default())
            .await
    }
}
