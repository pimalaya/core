pub mod config;
#[cfg(feature = "sendmail")]
pub mod sendmail;
#[cfg(feature = "smtp")]
pub mod smtp;

use async_trait::async_trait;

use super::add::AddMessage;
use crate::{account::config::HasAccountConfig, flag::Flag, folder::SENT, AnyResult};

#[async_trait]
pub trait SendMessage: Send + Sync {
    /// Send the given raw email message.
    async fn send_message(&self, msg: &[u8]) -> AnyResult<()>;
}

#[async_trait]
pub trait SendMessageThenSaveCopy: HasAccountConfig + AddMessage + SendMessage {
    /// Send the given raw email message, then save a copy to the Sent
    /// folder.
    async fn send_message_then_save_copy(&self, msg: &[u8]) -> AnyResult<()> {
        self.send_message(msg).await?;

        if self.account_config().should_save_copy_sent_message() {
            self.add_message_with_flag(SENT, msg, Flag::Seen).await?;
        }

        Ok(())
    }
}

impl<T: HasAccountConfig + AddMessage + SendMessage> SendMessageThenSaveCopy for T {}
