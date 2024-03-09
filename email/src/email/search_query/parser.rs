//! # Search emails query string parser
//!
//! This module contains parsers needed to parse a full search emails
//! query string. See [`filter::parser::filters`] for the filter query
//! string API, and [`sorter::parser::sorters`] for the sort query
//! string API.
//!
//! Parsing is based on the great lib [`chumsky`].

use chumsky::{error::Rich, extra, Parser};
use thiserror::Error;

use super::{
    filter::{self, SearchEmailsQueryFilter},
    sorter::{self, SearchEmailsQuerySorter},
    SearchEmailsQuery,
};

/// Error dedicated to search emails query parsing.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse search emails query `{1}`")]
    ParseError(Vec<Rich<'static, char>>, String),
}

/// Alias for a rich [`chumsky`] error for better diagnosis.
pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

/// Search emails full query parser.
///
/// Parses the given string into a [`SearchEmailsQuery`]. Because of
/// the recursive nature of [`SearchEmailsQueryFilter`], it is not
/// possible to directly parse a full query from a string using
/// [`chumsky`]. Instead the string is splitted in two, and filters
/// and sorters are parsed separately using [`parse_filters`] and
/// [`parse_sorters`].
///
/// A search emails query string can contain only a filter query, or
/// only a sorter query, or both together. In this last case, the
/// filter query needs to be defined first, then the sorter
/// query. They should be separated by the keyword `"order by"`.
///
/// See [`filter::parser::filters`] for more details on the filter
/// query string API, and [`sorter::parser::sorters`] for more details
/// on the sort query API.
///
/// # Examples
///
/// ```rust
/// use email::search_query::parser::parse;
///
/// pub fn main() {
///     // filter only
///     let query = "subject s and body b";
///     assert!(parse(query).is_ok());
///
///     // sort only
///     let query = "order by date desc";
///     assert!(parse(query).is_ok());
///
///     // filter then sort
///     let query = "subject s and body b order by date desc";
///     assert!(parse(query).is_ok());
/// }
/// ```
pub fn parse<'a>(input: impl AsRef<str> + 'a) -> Result<SearchEmailsQuery, Error> {
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

/// Search emails filters query parser.
///
/// Parses the given string into a recursive
/// [`SearchEmailsQueryFilter`].
///
/// If you want to parse a full search query, see [`parse`].
///
/// See [`filter::parser::filters`] for more details on the filter
/// query string API.
///
/// # Examples
///
/// ```rust
/// use email::search_query::parser::parse_filters;
///
/// pub fn main() {
///     let query = "subject s and body b";
///     assert!(parse_filters(query).is_ok());
/// }
/// ```
pub fn parse_filters<'a>(input: impl AsRef<str> + 'a) -> Result<SearchEmailsQueryFilter, Error> {
    let input = input.as_ref().trim();

    filter::parser::filters()
        .parse(input)
        .into_result()
        .map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, input.to_owned())
        })
}

/// Search emails sorters query parser.
///
/// Parses the given string into a list of
/// [`SearchEmailsQuerySorter`].
///
/// If you want to parse a full search query, see [`parse`].
///
/// See [`sorter::parser::sorters`] for more details on the sort query
/// string API.
///
/// # Examples
///
/// ```rust
/// use email::search_query::parser::parse_sorters;
///
/// pub fn main() {
///     let query = "date desc subject from";
///     assert!(parse_sorters(query).is_ok());
/// }
/// ```
pub fn parse_sorters<'a>(
    input: impl AsRef<str> + 'a,
) -> Result<Vec<SearchEmailsQuerySorter>, Error> {
    let input = input.as_ref().trim();

    sorter::parser::sorters()
        .parse(input)
        .into_result()
        .map_err(|errs| {
            let errs = errs
                .into_iter()
                .map(|err| err.clone().into_owned())
                .collect();
            Error::ParseError(errs, input.to_owned())
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
            super::parse("from f and to t").unwrap(),
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
            super::parse("order by from").unwrap(),
            SearchEmailsQuery {
                filters: None,
                sorters: Some(vec![From.into()])
            },
        );

        assert_eq!(
            super::parse("order by from asc subject desc").unwrap(),
            SearchEmailsQuery {
                filters: None,
                sorters: Some(vec![From.into(), (Subject, Descending).into()])
            },
        );
    }

    #[test]
    fn full() {
        assert_eq!(
            super::parse("from f and to t order by from to desc").unwrap(),
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
