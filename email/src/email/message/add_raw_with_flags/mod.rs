use async_trait::async_trait;

use crate::{email::Flags, Result};

#[cfg(feature = "imap-backend")]
pub mod imap;

#[async_trait]
pub trait AddRawMessageWithFlags: Send + Sync {
    /// Add the given raw email message with the given flags to the
    /// given folder.
    async fn add_raw_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<String>;
}
