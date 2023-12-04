use async_trait::async_trait;

use crate::{
    account::config::AccountConfig,
    envelope::Id,
    flag::{add::AddFlags, Flag},
    Result,
};

use super::move_::MoveMessages;

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
impl<T: MoveMessages + AddFlags> DeleteMessages for (AccountConfig, T) {
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        default_delete_messages(&self.0, &self.1, &self.1, folder, id).await
    }
}

pub async fn default_delete_messages(
    account_config: &AccountConfig,
    a: &dyn MoveMessages,
    b: &dyn AddFlags,
    folder: &str,
    id: &Id,
) -> Result<()> {
    let trash_folder = account_config.trash_folder_alias()?;

    if account_config.get_folder_alias(folder)? == trash_folder {
        b.add_flag(folder, id, Flag::Deleted).await
    } else {
        a.move_messages(folder, &trash_folder, id).await
    }
}
