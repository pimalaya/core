#[cfg(feature = "imap-backend")]
pub mod imap;

use async_trait::async_trait;

use crate::{
    account::AccountConfig,
    email::{envelope::Id, flag::AddFlags, Flag, Flags},
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

pub struct DefaultDeleteMessages {
    account_config: AccountConfig,
    move_messages: Box<dyn MoveMessages>,
    add_flags: Box<dyn AddFlags>,
}

#[async_trait]
impl MoveMessages for DefaultDeleteMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        self.move_messages
            .move_messages(from_folder, to_folder, id)
            .await
    }
}

#[async_trait]
impl AddFlags for DefaultDeleteMessages {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        self.add_flags.add_flags(folder, id, flags).await
    }
}

impl DefaultDeleteMessages {
    pub fn new(
        account_config: AccountConfig,
        move_messages: Box<dyn MoveMessages>,
        add_flags: Box<dyn AddFlags>,
    ) -> Box<dyn DeleteMessages> {
        Box::new(Self {
            account_config,
            move_messages,
            add_flags,
        })
    }
}

#[async_trait]
impl DeleteMessages for DefaultDeleteMessages {
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        default_delete_messages(&self.account_config, self, self, folder, id).await
    }
}

#[async_trait]
impl<T: MoveMessages + AddFlags> DeleteMessages for (AccountConfig, T) {
    async fn delete_messages(&self, folder: &str, id: &Id) -> Result<()> {
        default_delete_messages(&self.0, &self.1, &self.1, folder, id).await
    }
}

async fn default_delete_messages(
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
