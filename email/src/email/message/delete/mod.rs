pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use super::r#move::MoveMessages;
use crate::{
    account::config::HasAccountConfig,
    envelope::Id,
    flag::{add::AddFlags, Flag},
    folder::TRASH,
    AnyResult,
};

/// Feature to delete message(s).
#[async_trait]
pub trait DeleteMessages: Send + Sync {
    /// Delete messages from the given folder matching the given
    /// envelope id(s).
    ///
    /// This function should not definitely delete messages. Instead,
    /// if the message is in the Trash folder or if the delete message
    /// style matches the flag-based one, it should add the
    /// Deleted. Otherwise it should move the message to the Trash
    /// folder. Only [`ExpungeFolder`](crate::folder::ExpungeFolder)
    /// can definitely delete messages.
    async fn delete_messages(&self, folder: &str, id: &Id) -> AnyResult<()>;
}

/// Default backend feature to delete message(s).
///
/// This trait implements a default delete messages based on move
/// messages and add flags feature.
#[async_trait]
pub trait DefaultDeleteMessages: Send + Sync + HasAccountConfig + MoveMessages + AddFlags {
    async fn default_delete_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        let config = self.account_config();

        if config.is_trash_folder(folder) || config.is_delete_message_style_flag() {
            self.add_flag(folder, id, Flag::Deleted).await
        } else {
            self.move_messages(folder, TRASH, id).await
        }
    }
}

#[async_trait]
impl<T: DefaultDeleteMessages> DeleteMessages for T {
    async fn delete_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        self.default_delete_messages(folder, id).await
    }
}
