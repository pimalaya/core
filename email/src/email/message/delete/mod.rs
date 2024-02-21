#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::{
    account::config::HasAccountConfig,
    envelope::Id,
    flag::{add::AddFlags, Flag},
    Result,
};

use super::r#move::MoveMessages;

/// Delete messages backend feature.
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

/// Default delete messages backend feature.
///
/// This trait implements a default delete messages based on move
/// messages and add flags feature.
#[async_trait]
pub trait DefaultDeleteMessages: Send + Sync + HasAccountConfig + MoveMessages + AddFlags {
    async fn default_delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        let account_config = self.account_config();
        let trash_folder = account_config.get_trash_folder_alias();

        if account_config.get_folder_alias(folder) == trash_folder {
            self.add_flag(folder, id, Flag::Deleted).await
        } else {
            self.move_messages(folder, &trash_folder, id).await
        }
    }
}

#[async_trait]
impl<T: DefaultDeleteMessages> DeleteMessages for T {
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        self.default_delete_messages(folder, id).await
    }
}
