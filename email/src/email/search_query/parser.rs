//! # Search emails query parser
//!
//! This module contains parsers needed to parse a full search emails
//! query, and exposes a [`query`] parser. Parsing is based on the
//! great lib [chumsky].

use chumsky::prelude::*;
use thiserror::Error;

use crate::Result;

use super::{
    filter::{self, SearchEmailsQueryFilter},
    sorter::{self, SearchEmailsQuerySorter},
    SearchEmailsQuery,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse search emails query `{1}`")]
    ParseError(Vec<Rich<'static, char>>, String),
}

pub(crate) type ParserError<'a> = extra::Err<Rich<'a, char>>;

pub(crate) fn parse_query<'a>(input: impl AsRef<str> + 'a) -> Result<SearchEmailsQuery> {
    let input = input.as_ref().trim();

    if let Some((filters_input, sorters_input)) = input.rsplit_once("order by") {
        if filters_input.trim().is_empty() {
            let filters = None;
            let sorters = parse_sorters(sorters_input).map(Some)?;
            Ok(SearchEmailsQuery { filters, sorters })
        } else {
            let filters = parse_filters(filters_input).map(Some)?;
            let sorters = parse_sorters(sorters_input).map(Some)?;
            Ok(SearchEmailsQuery { filters, sorters })
        }
    } else {
        let filters = parse_filters(input).map(Some)?;
        let sorters = None;
        Ok(SearchEmailsQuery { filters, sorters })
    }
}

pub(crate) fn parse_filters<'a>(input: impl AsRef<str> + 'a) -> Result<SearchEmailsQueryFilter> {
    let input = input.as_ref().trim();

    filter::parser::filters()
        .parse(input)
        .into_result()
        .map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, input.to_owned()).into()
        })
}

pub(crate) fn parse_sorters<'a>(
    input: impl AsRef<str> + 'a,
) -> Result<Vec<SearchEmailsQuerySorter>> {
    let input = input.as_ref().trim();

    sorter::parser::sorters()
        .parse(input)
        .into_result()
        .map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, input.to_owned()).into()
        })
}

#[cfg(test)]
mod tests {
    use crate::search_query::{
        filter::SearchEmailsQueryFilter,
        sorter::{SearchEmailsQueryOrder::*, SearchEmailsQuerySorterKind::*},
        SearchEmailsQuery,
    };

    #[test]
    fn filters_only() {
        assert_eq!(
            super::parse_query("from f and to t").unwrap(),
            SearchEmailsQuery {
                filters: Some(SearchEmailsQueryFilter::And(
                    Box::new(SearchEmailsQueryFilter::From("f".into())),
                    Box::new(SearchEmailsQueryFilter::To("t".into()))
                )),
                sorters: None,
            },
        );
    }

    #[test]
    fn sorters_only() {
        assert_eq!(
            super::parse_query("order by from").unwrap(),
            SearchEmailsQuery {
                filters: None,
                sorters: Some(vec![From.into()])
            },
        );

        assert_eq!(
            super::parse_query("order by from asc subject desc").unwrap(),
            SearchEmailsQuery {
                filters: None,
                sorters: Some(vec![From.into(), (Subject, Descending).into()])
            },
        );
    }

    #[test]
    fn query() {
        assert_eq!(
            super::parse_query("from f and to t order by from to desc").unwrap(),
            SearchEmailsQuery {
                filters: Some(SearchEmailsQueryFilter::And(
                    Box::new(SearchEmailsQueryFilter::From("f".into())),
                    Box::new(SearchEmailsQueryFilter::To("t".into()))
                )),
                sorters: Some(vec![From.into(), (To, Descending).into()])
            },
        );
    }
}
