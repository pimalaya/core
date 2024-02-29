use chrono::{DateTime, Local};
use chumsky::{error::Rich, Parser};
use std::str::FromStr;
use thiserror::Error;

pub(crate) mod parsers;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse search emails query `{1}`")]
    ParseError(Vec<Rich<'static, char>>, String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SearchEmailsQuery {
    And(Box<SearchEmailsQuery>, Box<SearchEmailsQuery>),
    Or(Box<SearchEmailsQuery>, Box<SearchEmailsQuery>),
    Not(Box<SearchEmailsQuery>),
    Before(DateTime<Local>),
    After(DateTime<Local>),
    From(String),
    To(String),
    Subject(String),
    Body(String),
    Keyword(String),
}

// TODO: merge with SearchEmailsQuery
// #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
// pub enum ListEnvelopesComparator {
//     Descending(Box<ListEnvelopesComparator>),
//     SentAt,
//     ReceivedAt,
//     From,
//     To,
//     Subject,
// }

impl FromStr for SearchEmailsQuery {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        parsers::query().parse(s).into_result().map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, s.to_owned())
        })
    }
}
