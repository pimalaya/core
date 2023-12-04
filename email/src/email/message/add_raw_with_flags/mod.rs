use async_trait::async_trait;

use crate::{
    envelope::{Id, SingleId},
    flag::{add::AddFlags, Flag, Flags},
    Result,
};

use super::add_raw::AddRawMessage;

#[cfg(feature = "imap")]
pub mod imap;
pub mod maildir;

#[async_trait]
pub trait AddRawMessageWithFlags: Send + Sync {
    /// Add the given raw email message with the given flags to the
    /// given folder.
    async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId>;

    /// Add the given raw email message with the given flag to the
    /// given folder.
    async fn add_raw_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<SingleId> {
        self.add_raw_message_with_flags(folder, raw_msg, &Flags::from_iter([flag]))
            .await
    }
}

#[async_trait]
impl<T: AddRawMessage + AddFlags> AddRawMessageWithFlags for T {
    async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        let id = self.add_raw_message(folder, raw_msg).await?;
        self.add_flags(folder, &Id::from(&id), flags).await?;
        Ok(id)
    }
}
