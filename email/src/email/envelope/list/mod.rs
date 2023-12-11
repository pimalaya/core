pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
pub mod maildir;

use async_trait::async_trait;

use crate::Result;

use super::Envelopes;

#[async_trait]
pub trait ListEnvelopes: Send + Sync {
    /// List all available envelopes from the given folder matching
    /// the given pagination.
    async fn list_envelopes(
        &self,
        folder: &str,
        page_size: usize,
        page: usize,
    ) -> Result<Envelopes>;
}
