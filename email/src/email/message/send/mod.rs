pub mod config;
#[cfg(feature = "sendmail")]
pub mod sendmail;
#[cfg(feature = "smtp")]
pub mod smtp;

use async_trait::async_trait;

use crate::Result;

#[async_trait]
pub trait SendMessage: Send + Sync {
    /// Send the given raw email message.
    async fn send_message(&self, msg: &[u8]) -> Result<()>;
}
