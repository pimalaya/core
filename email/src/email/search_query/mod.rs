pub mod filter;
pub(crate) mod parser;
pub mod sorter;

use std::str::FromStr;

use crate::Error;

use self::{filter::SearchEmailsQueryFilter, sorter::SearchEmailsQuerySorter};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuery {
    pub filters: Option<SearchEmailsQueryFilter>,
    pub sorters: Option<Vec<SearchEmailsQuerySorter>>,
}

impl FromStr for SearchEmailsQuery {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parser::parse_query(s)
    }
}
