//! # Search emails query sorters parser
//!
//! This module contains parsers needed to parse a full search emails
//! query, and exposes a [`query`] parser. Parsing is based on the
//! great lib [`chumsky`].

use chrono::{DateTime, Local, ParseError};
use chumsky::prelude::*;
use thiserror::Error;

use crate::search_query::parser::ParserError;

use super::{SearchEmailsQueryOrder, SearchEmailsQuerySorter, SearchEmailsQuerySorterKind};

/// Error dedicated to search emails query parsing.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse date from list envelopes query")]
    ParseNaiveDateTimeError(#[source] ParseError),
    #[error("cannot parse date from list envelopes query: cannot apply local timezone to {0}")]
    ParseLocalDateTimeError(String),
    #[error("cannot parse date from list envelopes query: cannot choose between {0} and {1}")]
    ParseLocalDateTimeAmbiguousError(DateTime<Local>, DateTime<Local>),
}

pub(crate) fn sorters<'a>(
) -> impl Parser<'a, &'a str, Vec<SearchEmailsQuerySorter>, ParserError<'a>> + Clone {
    choice((date(), from(), to(), subject()))
        .separated_by(
            just(' ')
                .labelled("space between sorters")
                .repeated()
                .at_least(1),
        )
        .collect()
}

fn date<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        date_kind()
            .then(
                just(' ')
                    .labelled("space after `date`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsQuerySorter::from),
        date_kind().map(SearchEmailsQuerySorter::from),
    ))
}

fn date_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorterKind, ParserError<'a>> + Clone
{
    just('d')
        .labelled("`date`")
        .ignored()
        .then_ignore(just('a').labelled("`date`"))
        .then_ignore(just('t').labelled("`date`"))
        .then_ignore(just('e').labelled("`date`"))
        .to(SearchEmailsQuerySorterKind::Date)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        from_kind()
            .then(
                just(' ')
                    .labelled("space after `from`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsQuerySorter::from),
        from_kind().map(SearchEmailsQuerySorter::from),
    ))
}

fn from_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorterKind, ParserError<'a>> + Clone
{
    just('f')
        .labelled("`from`")
        .ignored()
        .then_ignore(just('r').labelled("`from`"))
        .then_ignore(just('o').labelled("`from`"))
        .then_ignore(just('m').labelled("`from`"))
        .to(SearchEmailsQuerySorterKind::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        to_kind()
            .then(
                just(' ')
                    .labelled("space after `to`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsQuerySorter::from),
        to_kind().map(SearchEmailsQuerySorter::from),
    ))
}

fn to_kind<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorterKind, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to`")
        .ignored()
        .then_ignore(just('o').labelled("`to`"))
        .to(SearchEmailsQuerySorterKind::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        subject_kind()
            .then(
                just(' ')
                    .labelled("space after `subject`")
                    .repeated()
                    .at_least(1)
                    .ignore_then(choice((ascending(), descending()))),
            )
            .map(SearchEmailsQuerySorter::from),
        subject_kind().map(SearchEmailsQuerySorter::from),
    ))
}

fn subject_kind<'a>(
) -> impl Parser<'a, &'a str, SearchEmailsQuerySorterKind, ParserError<'a>> + Clone {
    just('s')
        .labelled("`subject`")
        .ignored()
        .then_ignore(just('u').labelled("`subject`"))
        .then_ignore(just('b').labelled("`subject`"))
        .then_ignore(just('j').labelled("`subject`"))
        .then_ignore(just('e').labelled("`subject`"))
        .then_ignore(just('c').labelled("`subject`"))
        .then_ignore(just('t').labelled("`subject`"))
        .to(SearchEmailsQuerySorterKind::Subject)
}

fn ascending<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryOrder, ParserError<'a>> + Clone {
    just('a')
        .labelled("`asc`")
        .ignore_then(just('s').labelled("`asc`"))
        .ignore_then(just('c').labelled("`asc`"))
        .to(SearchEmailsQueryOrder::Ascending)
}

fn descending<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryOrder, ParserError<'a>> + Clone {
    just('d')
        .labelled("`desc`")
        .ignore_then(just('e').labelled("`desc`"))
        .ignore_then(just('s').labelled("`desc`"))
        .ignore_then(just('c').labelled("`desc`"))
        .to(SearchEmailsQueryOrder::Descending)
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use super::{
        SearchEmailsQueryOrder::*, SearchEmailsQuerySorter, SearchEmailsQuerySorterKind::*,
    };

    #[test]
    fn empty_sorter() {
        assert_eq!(super::sorters().parse("").into_result(), Ok(vec![]));
    }

    #[test]
    fn simple_sorters() {
        assert_eq!(
            super::sorters().parse("date from to").into_result(),
            Ok(vec![
                SearchEmailsQuerySorter(Date, Ascending),
                SearchEmailsQuerySorter(From, Ascending),
                SearchEmailsQuerySorter(To, Ascending)
            ])
        );
    }

    #[test]
    fn mixed_sorters() {
        assert_eq!(
            super::sorters()
                .parse("date asc from subject desc")
                .into_result(),
            Ok(vec![
                SearchEmailsQuerySorter(Date, Ascending),
                SearchEmailsQuerySorter(From, Ascending),
                SearchEmailsQuerySorter(Subject, Descending)
            ])
        );
    }
}