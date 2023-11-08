use async_trait::async_trait;

use crate::{
    account::WithAccountConfig,
    email::{envelope::Id, flag::AddFlags, Flag},
    Result,
};

use super::MoveMessages;

#[async_trait]
pub trait DeleteMessages: Send + Sync {
    /// Delete emails from the given folder to the given folder
    /// matching the given id.
    ///
    /// This function should not definitely delete messages. Instead,
    /// if the message is in the Trash folder, it should add the
    /// [`Flag::Deleted`](crate::email::Flag). Otherwise it should
    /// move the message to the Trash folder. Only
    /// [`ExpungeFolder`](crate::folder::ExpungeFolder) can definitely
    /// delete messages.
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()>;
}

#[async_trait]
impl<T: WithAccountConfig + MoveMessages + AddFlags> DeleteMessages for T {
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        let trash_folder = self.account_config().trash_folder_alias()?;

        if self.account_config().get_folder_alias(folder)? == trash_folder {
            self.add_flag(folder, id, Flag::Deleted).await
        } else {
            self.move_messages(folder, &trash_folder, id).await
        }
    }
}
