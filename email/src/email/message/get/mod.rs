use async_trait::async_trait;

use crate::{email::envelope::MultipleIds, Result};

use super::Messages;

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
