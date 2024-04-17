//! # Search emails query string parser
//!
//! This module contains parsers needed to parse a full search emails
//! query from a string slice. See [`filter::parser::query`] and
//! [`sort::parser::query`] for more details.
//!
//! Parsing is based on the great lib [`chumsky`].

use chumsky::{error::Rich, extra, Parser};

use super::{
    error::Error,
    filter::{self, SearchEmailsFilterQuery},
    sort::{self, SearchEmailsSorter},
    SearchEmailsQuery,
};

/// Alias for a rich [`chumsky`] error for better diagnosis.
pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

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
pub fn parse<'a>(input: impl AsRef<str> + 'a) -> Result<SearchEmailsQuery, Error> {
    let input = input.as_ref().trim();

    if let Some((filters_input, sorters_input)) = input.rsplit_once("order by") {
        if filters_input.trim().is_empty() {
            let filter = None;
            let sort = parse_sort(sorters_input).map(Some)?;
            Ok(SearchEmailsQuery { filter, sort })
        } else {
            let filter = parse_filter(filters_input).map(Some)?;
            let sort = parse_sort(sorters_input).map(Some)?;
            Ok(SearchEmailsQuery { filter, sort })
        }
    } else {
        let filter = parse_filter(input).map(Some)?;
        let sort = None;
        Ok(SearchEmailsQuery { filter, sort })
    }
}

/// Parse the given string into a [`SearchEmailsFilterQuery`].
///
/// See [`filter::parser::query`] for more details.
pub fn parse_filter<'a>(input: impl AsRef<str> + 'a) -> Result<SearchEmailsFilterQuery, Error> {
    let input = input.as_ref().trim();

    filter::parser::query()
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

/// Parse the given string into a list of [`SearchEmailsSorter`].
///
/// See [`sort::parser::query`] for more details.
pub fn parse_sort<'a>(input: impl AsRef<str> + 'a) -> Result<Vec<SearchEmailsSorter>, Error> {
    let input = input.as_ref().trim();

    sort::parser::query()
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
