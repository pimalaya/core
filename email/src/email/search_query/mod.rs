//! # Search emails query
//!
//! This module exposes the [`SearchEmailsQuery`] structure, which
//! allows you to filter and sort emails. A query can be parsed from a
//! query string, see the [`parser`] module for more details. See also
//! the [`filter`] module and the [`sorter`] module.

pub mod filter;
pub mod parser;
pub mod sorter;

use std::str::FromStr;

use self::{filter::SearchEmailsQueryFilter, parser::Error, sorter::SearchEmailsQuerySorter};

/// The search emails query structure.
///
/// The query is composed of a recursive [`SearchEmailsQueryFilter`]
/// and a list of [`SearchEmailsQuerySorter`]s.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuery {
    /// The recursive emails search query filter.
    pub filters: Option<SearchEmailsQueryFilter>,

    /// The list of emails search query sorters.
    pub sorters: Option<Vec<SearchEmailsQuerySorter>>,
}

impl FromStr for SearchEmailsQuery {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parser::parse(s)
    }
}
