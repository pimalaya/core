use async_trait::async_trait;

use crate::{
    email::{envelope::Id, flag::add::AddFlags, Flags},
    Result,
};

use super::add_raw::AddRawMessage;

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddRawMessageWithFlags: Send + Sync {
    /// Add the given raw email message with the given flags to the
    /// given folder.
    async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<Id>;
}

#[async_trait]
impl<T: AddRawMessage + AddFlags> AddRawMessageWithFlags for T {
    async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<Id> {
        let id = self.add_raw_message(folder, raw_msg).await?;
        self.add_flags(folder, &id, flags).await?;
        Ok(id)
    }
}
