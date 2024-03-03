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
    search_query::sorter::{SearchEmailsQueryOrder, SearchEmailsQuerySorter},
    Result,
};

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

impl ListEnvelopesOptions {
    pub fn sort_envelopes(&self, envelopes: &mut Envelopes) {
        envelopes.sort_by(|a, b| {
            if let Some(sorters) = self.query.as_ref().and_then(|q| q.sorters.as_ref()) {
                for sorter in sorters {
                    match sorter {
                        SearchEmailsQuerySorter::Date(order) => {
                            match (a.date.cmp(&b.date), order) {
                                (Ordering::Equal, _) => {
                                    continue;
                                }
                                (order, SearchEmailsQueryOrder::Ascending) => {
                                    return order;
                                }
                                (order, SearchEmailsQueryOrder::Descending) => {
                                    return order.reverse();
                                }
                            }
                        }
                        SearchEmailsQuerySorter::From(order) => {
                            match (a.from.cmp(&b.from), order) {
                                (Ordering::Equal, _) => {
                                    continue;
                                }
                                (order, SearchEmailsQueryOrder::Ascending) => {
                                    return order;
                                }
                                (order, SearchEmailsQueryOrder::Descending) => {
                                    return order.reverse();
                                }
                            }
                        }
                        SearchEmailsQuerySorter::To(order) => match (a.to.cmp(&b.to), order) {
                            (Ordering::Equal, _) => {
                                continue;
                            }
                            (order, SearchEmailsQueryOrder::Ascending) => {
                                return order;
                            }
                            (order, SearchEmailsQueryOrder::Descending) => {
                                return order.reverse();
                            }
                        },
                        SearchEmailsQuerySorter::Subject(order) => {
                            match (a.subject.cmp(&b.subject), order) {
                                (Ordering::Equal, _) => {
                                    continue;
                                }
                                (order, SearchEmailsQueryOrder::Ascending) => {
                                    return order;
                                }
                                (order, SearchEmailsQueryOrder::Descending) => {
                                    return order.reverse();
                                }
                            }
                        }
                    }
                }
            }

            b.date.cmp(&a.date)
        });
    }
}
