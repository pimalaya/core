pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::{
    envelope::Id,
    flag::{add::AddFlags, Flag},
    Result,
};

use super::{peek::PeekMessages, Messages};

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

// #[async_trait]
// impl<T: PeekMessages + AddFlags> GetMessages for T {
//     async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
//         default_get_messages(self, self, folder, id).await
//     }
// }

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
