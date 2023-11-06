use async_trait::async_trait;

use crate::{
    email::{envelope::Id, Flag, Flags},
    Result,
};

use super::add_raw_with_flags::AddRawMessageWithFlags;

#[async_trait]
pub trait AddRawMessageWithFlag: Send + Sync {
    /// Add the given raw email message with the given flag to the
    /// given folder.
    async fn add_raw_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<Id>;
}

#[async_trait]
impl<U: AddRawMessageWithFlags> AddRawMessageWithFlag for U {
    async fn add_raw_message_with_flag(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flag: Flag,
    ) -> Result<Id> {
        self.add_raw_message_with_flags(folder, raw_msg, &Flags::from_iter([flag]))
            .await
    }
}
