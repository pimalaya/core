pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;
use chrono::{DateTime, FixedOffset};
use std::collections::BTreeSet;

use crate::Result;

use super::Envelopes;

#[async_trait]
pub trait ListEnvelopes: Send + Sync {
    /// List all available envelopes from the given folder matching
    /// the given pagination.
    async fn list_envelopes(&self, folder: &str, opts: ListEnvelopesOptions) -> Result<Envelopes>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListEnvelopesOptions {
    page_size: usize,
    page: usize,
    filter: Option<ListEnvelopesCondition>,
    sort: Vec<ListEnvelopesComparator>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ListEnvelopesCondition {
    And(BTreeSet<ListEnvelopesCondition>),
    Or(BTreeSet<ListEnvelopesCondition>),
    Not(BTreeSet<ListEnvelopesCondition>),
    Folder(String),
    Before(DateTime<FixedOffset>),
    After(DateTime<FixedOffset>),
    From(String),
    To(String),
    Subject(String),
    Body(String),
    Keyword(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ListEnvelopesComparator {
    Descending(Box<ListEnvelopesComparator>),
    SentAt,
    ReceivedAt,
    From,
    To,
    Subject,
}
