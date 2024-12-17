pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use super::{peek::PeekMessages, Messages};
use crate::{
    envelope::Id,
    flag::{add::AddFlags, Flag},
    AnyResult,
};

/// Get messages feature.
#[async_trait]
pub trait GetMessages: Send + Sync {
    /// Get email messages from the given folder matching the given
    /// ids.
    ///
    /// When getting messages, the [`Flag::Seen`](crate::email::Flag)
    /// is added to the associated envelopes. If you do not want
    /// envelopes to change, see
    /// [`PeekMessages`](super::peek::PeekMessages).
    async fn get_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages>;
}

/// Default get messages backend feature.
///
/// This trait implements a default get messages based on peek
/// messages and add flags feature.
#[async_trait]
pub trait DefaultGetMessages: Send + Sync + PeekMessages + AddFlags {
    async fn default_get_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        let messages = self.peek_messages(folder, id).await?;
        self.add_flag(folder, id, Flag::Seen).await?;
        Ok(messages)
    }
}

#[async_trait]
impl<T: DefaultGetMessages> GetMessages for T {
    async fn get_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        self.default_get_messages(folder, id).await
    }
}
