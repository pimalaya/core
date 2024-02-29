pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;

use crate::{email::search_query::SearchEmailsQuery, Result};

use super::Envelopes;

#[async_trait]
pub trait ListEnvelopes: Send + Sync {
    /// List all available envelopes from the given folder matching
    /// the given pagination.
    async fn list_envelopes(&self, folder: &str, opts: ListEnvelopesOptions) -> Result<Envelopes>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListEnvelopesOptions {
    pub page_size: usize,
    pub page: usize,
    pub query: Option<SearchEmailsQuery>,
}
