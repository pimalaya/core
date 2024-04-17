//! # Search emails query
//!
//! This module exposes [`SearchEmailsQuery`], a structure that helps
//! you to filter and sort emails. A search emails query is composed
//! of a [`filter`] query and a [`sort`] query.
//!
//! It is actually used by
//! [`ListEnvelopesOptions`](crate::envelope::list::ListEnvelopesOptions)
//! to filter and sort envelopes.
//!
//! The search emails query can be parsed from a string via
//! [`FromStr`], see the [`parser`] module for more details.
//!
//! ```
#![doc = include_str!("../../../examples/search_emails_query.rs")]
//! ```

pub mod error;
pub mod filter;
pub mod parser;
pub mod sort;

use std::str::FromStr;

use error::Error;

use self::{filter::SearchEmailsFilterQuery, sort::SearchEmailsSortQuery};

/// The search emails query structure.
///
/// The query is composed of a recursive [`SearchEmailsFilterQuery`]
/// and a list of [`SearchEmailsSorter`]s.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuery {
    /// The recursive emails search filter query.
    pub filter: Option<SearchEmailsFilterQuery>,

    /// The emails search sort query.
    pub sort: Option<SearchEmailsSortQuery>,
}

/// Parse the given string slice into a [`SearchEmailsQuery`].
///
/// Because of the recursive nature of [`SearchEmailsFilterQuery`], it
/// is not possible to directly parse a full query from a string using
/// [`chumsky`]. Instead the string is splitted in two, and filters
/// and sorters are parsed separately.
///
/// A search emails query string can contain a filter query, a sorter
/// query or both. In this last case, the filter query needs to be
/// defined first, then the sorter query. They should be separated by
/// the keyword `"order by"`.
///
/// See [`filter::parser::query`] for more details on the filter query
/// string API, and [`sort::parser::query`] for more details on the
/// sort query API.
///
/// # Examples
///
/// ```rust
/// use email::search_query::SearchEmailsQuery;
/// use std::str::FromStr;
///
/// pub fn main() {
///     // filter only
///     "subject foo and body bar".parse::<SearchEmailsQuery>().unwrap();
///
///     // sort only
///     "order by date desc".parse::<SearchEmailsQuery>().unwrap();
///
///     // filter then sort
///     "subject foo and body bar order by subject".parse::<SearchEmailsQuery>().unwrap();
/// }
/// ```
///
/// # ABNF
///
/// ```abnf,ignore
/// query = filter-query / "order by" SP sort-query / filter-query SP "order by" SP sort-query
#[doc = include_str!("./filter/grammar.abnf")]
///
#[doc = include_str!("./sort/grammar.abnf")]
/// ```
impl FromStr for SearchEmailsQuery {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::search_query::{
        filter::SearchEmailsFilterQuery,
        sort::{SearchEmailsSorterKind::*, SearchEmailsSorterOrder::*},
        SearchEmailsQuery,
    };

    #[test]
    fn filters_only() {
        assert_eq!(
            "from f and to t".parse::<SearchEmailsQuery>().unwrap(),
            SearchEmailsQuery {
                filter: Some(SearchEmailsFilterQuery::And(
                    Box::new(SearchEmailsFilterQuery::From("f".into())),
                    Box::new(SearchEmailsFilterQuery::To("t".into()))
                )),
                sort: None,
            },
        );
    }

    #[test]
    fn sorters_only() {
        assert_eq!(
            "order by from".parse::<SearchEmailsQuery>().unwrap(),
            SearchEmailsQuery {
                filter: None,
                sort: Some(vec![From.into()])
            },
        );

        assert_eq!(
            "order by from asc subject desc"
                .parse::<SearchEmailsQuery>()
                .unwrap(),
            SearchEmailsQuery {
                filter: None,
                sort: Some(vec![From.into(), (Subject, Descending).into()])
            },
        );
    }

    #[test]
    fn full() {
        assert_eq!(
            "from f and to t order by from to desc"
                .parse::<SearchEmailsQuery>()
                .unwrap(),
            SearchEmailsQuery {
                filter: Some(SearchEmailsFilterQuery::And(
                    Box::new(SearchEmailsFilterQuery::From("f".into())),
                    Box::new(SearchEmailsFilterQuery::To("t".into()))
                )),
                sort: Some(vec![From.into(), (To, Descending).into()])
            },
        );
    }
}
