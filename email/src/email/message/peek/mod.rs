#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use super::Messages;
use crate::{envelope::Id, AnyResult};

#[async_trait]
pub trait PeekMessages: Send + Sync {
    /// Peek email messages from the given folder matching the given
    /// ids.
    ///
    /// When peeking messages, associated envelope flags do not
    /// change. If you want [`Flag::Seen`](crate::email::Flag) to be
    /// automatically added to envelopes, see
    /// [`GetMessages`](super::get::GetMessages).
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages>;
}
