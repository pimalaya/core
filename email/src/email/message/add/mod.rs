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
    AnyResult,
};

#[async_trait]
pub trait AddMessage: Send + Sync {
    /// Add the given raw email message with the given flags to the
    /// given folder.
    async fn add_message_with_flags(
        &self,
        folder: &str,
        msg: &[u8],
        flags: &Flags,
    ) -> AnyResult<SingleId>;

    /// Add the given raw email message with the given flag to the
    /// given folder.
    async fn add_message_with_flag(
        &self,
        folder: &str,
        msg: &[u8],
        flag: Flag,
    ) -> AnyResult<SingleId> {
        self.add_message_with_flags(folder, msg, &Flags::from_iter([flag]))
            .await
    }

    /// Add the given raw email message to the given folder.
    async fn add_message(&self, folder: &str, msg: &[u8]) -> AnyResult<SingleId> {
        self.add_message_with_flags(folder, msg, &Default::default())
            .await
    }
}
