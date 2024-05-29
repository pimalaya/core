#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use super::{Envelope, SingleId};
use crate::AnyResult;

#[async_trait]
pub trait GetEnvelope: Send + Sync {
    /// Get the envelope from the given folder matching the given id.
    async fn get_envelope(&self, folder: &str, id: &SingleId) -> AnyResult<Envelope>;
}
