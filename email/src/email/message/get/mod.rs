pub mod config;
#[cfg(feature = "imap")]
pub mod imap;

use async_trait::async_trait;

#[cfg(feature = "flag-add")]
use crate::flag::{add::AddFlags, Flag};
use crate::{envelope::Id, Result};

#[cfg(feature = "flag-add")]
use super::peek::PeekMessages;
use super::Messages;

#[async_trait]
pub trait GetMessages: Send + Sync {
    /// Get email messages from the given folder matching the given
    /// ids.
    ///
    /// When getting messages, the [`Flag::Seen`](crate::email::Flag)
    /// is added to the associated envelopes. If you do not want
    /// envelopes to change, see
    /// [`PeekMessages`](super::peek::PeekMessages).
    async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages>;
}

#[cfg(feature = "flag-add")]
#[async_trait]
impl<T: PeekMessages + AddFlags> GetMessages for T {
    async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        default_get_messages(self, self, folder, id).await
    }
}

#[cfg(feature = "flag-add")]
pub async fn default_get_messages(
    a: &dyn PeekMessages,
    b: &dyn AddFlags,
    folder: &str,
    id: &Id,
) -> Result<Messages> {
    let messages = a.peek_messages(folder, id).await?;
    b.add_flag(folder, id, Flag::Seen).await?;
    Ok(messages)
}
