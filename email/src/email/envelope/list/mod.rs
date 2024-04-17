pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use std::cmp::Ordering;

use async_trait::async_trait;

use super::{Envelope, Envelopes};
use crate::{
    email::search_query::SearchEmailsQuery,
    search_query::sort::{SearchEmailsSorter, SearchEmailsSorterKind, SearchEmailsSorterOrder},
    AnyResult,
};

#[async_trait]
pub trait ListEnvelopes: Send + Sync {
    /// List all available envelopes from the given folder matching
    /// the given pagination.
    async fn list_envelopes(
        &self,
        folder: &str,
        opts: ListEnvelopesOptions,
    ) -> AnyResult<Envelopes>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListEnvelopesOptions {
    pub page_size: usize,
    pub page: usize,
    pub query: Option<SearchEmailsQuery>,
}

impl SearchEmailsSorter {
    pub fn cmp_envelopes(&self, a: &Envelope, b: &Envelope) -> Ordering {
        use SearchEmailsSorterKind::*;
        use SearchEmailsSorterOrder::*;

        match self {
            SearchEmailsSorter(Date, Ascending) => a.date.cmp(&b.date),
            SearchEmailsSorter(Date, Descending) => b.date.cmp(&a.date),
            SearchEmailsSorter(From, Ascending) => a.from.cmp(&b.from),
            SearchEmailsSorter(From, Descending) => b.from.cmp(&a.from),
            SearchEmailsSorter(To, Ascending) => a.to.cmp(&b.to),
            SearchEmailsSorter(To, Descending) => b.to.cmp(&a.to),
            SearchEmailsSorter(Subject, Ascending) => a.subject.cmp(&b.subject),
            SearchEmailsSorter(Subject, Descending) => b.subject.cmp(&a.subject),
        }
    }
}

impl ListEnvelopesOptions {
    pub fn sort_envelopes(&self, envelopes: &mut Envelopes) {
        envelopes.sort_by(|a, b| {
            if let Some(sorters) = self.query.as_ref().and_then(|q| q.sort.as_ref()) {
                for sorter in sorters {
                    let cmp = sorter.cmp_envelopes(a, b);
                    if cmp.is_ne() {
                        return cmp;
                    }
                }
            }

            a.date.cmp(&b.date).reverse()
        });
    }
}
