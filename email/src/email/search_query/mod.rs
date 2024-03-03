pub mod filter;
pub(crate) mod parser;
pub mod sorter;

use chumsky::{error::Rich, Parser};
use std::str::FromStr;
use thiserror::Error;

use self::{filter::SearchEmailsQueryFilter, sorter::SearchEmailsQuerySorter};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse search emails query `{1}`")]
    ParseError(Vec<Rich<'static, char>>, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SearchEmailsQuery {
    pub filters: Option<SearchEmailsQueryFilter>,
    pub sorters: Option<Vec<SearchEmailsQuerySorter>>,
}

impl FromStr for SearchEmailsQuery {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parser::query().parse(s).into_result().map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, s.to_owned())
        })
    }
}
