use async_trait::async_trait;

use crate::Result;

use super::{Envelope, Id};

#[cfg(feature = "imap")]
pub mod imap;
pub mod maildir;

#[async_trait]
pub trait GetEnvelope: Send + Sync {
    /// Get the envelope from the given folder matching the given id.
    async fn get_envelope(&self, folder: &str, id: &Id) -> Result<Envelope>;
}
