pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;
use std::cmp::Ordering;

use crate::{
    email::search_query::SearchEmailsQuery,
    search_query::sorter::{
        SearchEmailsQueryOrder, SearchEmailsQuerySorter, SearchEmailsQuerySorterKind,
    },
    Result,
};

use super::{Envelope, Envelopes};

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

impl SearchEmailsQuerySorter {
    pub fn cmp_envelopes(&self, a: &Envelope, b: &Envelope) -> Ordering {
        use SearchEmailsQueryOrder::*;
        use SearchEmailsQuerySorterKind::*;

        match self {
            SearchEmailsQuerySorter(Date, Ascending) => a.date.cmp(&b.date),
            SearchEmailsQuerySorter(Date, Descending) => b.date.cmp(&a.date),
            SearchEmailsQuerySorter(From, Ascending) => a.from.cmp(&b.from),
            SearchEmailsQuerySorter(From, Descending) => b.from.cmp(&a.from),
            SearchEmailsQuerySorter(To, Ascending) => a.to.cmp(&b.to),
            SearchEmailsQuerySorter(To, Descending) => b.to.cmp(&a.to),
            SearchEmailsQuerySorter(Subject, Ascending) => a.subject.cmp(&b.subject),
            SearchEmailsQuerySorter(Subject, Descending) => b.subject.cmp(&a.subject),
        }
    }
}

impl ListEnvelopesOptions {
    pub fn sort_envelopes(&self, envelopes: &mut Envelopes) {
        envelopes.sort_by(|a, b| {
            if let Some(sorters) = self.query.as_ref().and_then(|q| q.sorters.as_ref()) {
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
