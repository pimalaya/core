use async_trait::async_trait;

use crate::{
    email::{
        envelope::{flag::AddFlags, MultipleIds},
        Flag, Flags,
    },
    Result,
};

use super::{Messages, PeekMessages};

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait GetMessages: Send + Sync {
    /// Get email messages from the given folder matching the given
    /// ids.
    ///
    /// When getting messages, the [`Flag::Seen`](crate::email::Flag)
    /// is added to the associated envelopes. If you do not want
    /// envelopes to change, see
    /// [`PeekMessages`](super::peek::PeekMessages).
    async fn get_messages(&self, folder: &str, ids: &MultipleIds) -> Result<Messages>;
}

#[async_trait]
impl<T: PeekMessages + AddFlags> GetMessages for T {
    async fn get_messages(&self, folder: &str, ids: &MultipleIds) -> Result<Messages> {
        let messages = self.peek_messages(folder, ids).await?;
        self.add_flags(folder, &ids.into(), &Flags::from_iter([Flag::Seen]))
            .await?;
        Ok(messages)
    }
}
