//! # Search emails query sorters parser
//!
//! This module contains parsers needed to parse a full search emails
//! query, and exposes a [`query`] parser. Parsing is based on the
//! great lib [`chumsky`].

use chrono::{DateTime, Local, ParseError};
use chumsky::prelude::*;
use thiserror::Error;

use crate::search_query::parser::ParserError;

use super::{SearchEmailsQueryOrder, SearchEmailsQuerySorter};

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
            space()
                .labelled("space between sorters")
                .repeated()
                .at_least(1),
        )
        .collect()
}

fn date<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        just('d')
            .labelled("`date`")
            .ignore_then(just('a').labelled("`date`"))
            .ignore_then(just('t').labelled("`date`"))
            .ignore_then(just('e').labelled("`date`"))
            .to(SearchEmailsQueryOrder::Ascending)
            .map(SearchEmailsQuerySorter::Date),
        date_asc(),
        date_desc(),
    ))
}

fn date_asc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('d')
        .labelled("`date:asc`")
        .ignore_then(just('a').labelled("`date:asc`"))
        .ignore_then(just('t').labelled("`date:asc`"))
        .ignore_then(just('e').labelled("`date:asc`"))
        .ignore_then(just(':').labelled("`date:asc`"))
        .ignore_then(just('a').labelled("`date:asc`"))
        .ignore_then(just('s').labelled("`date:asc`"))
        .ignore_then(just('c').labelled("`date:asc`"))
        .to(SearchEmailsQueryOrder::Ascending)
        .map(SearchEmailsQuerySorter::Date)
}

fn date_desc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('d')
        .labelled("`date:desc`")
        .ignore_then(just('a').labelled("`date:desc`"))
        .ignore_then(just('t').labelled("`date:desc`"))
        .ignore_then(just('e').labelled("`date:desc`"))
        .ignore_then(just(':').labelled("`date:desc`"))
        .ignore_then(just('d').labelled("`date:desc`"))
        .ignore_then(just('e').labelled("`date:desc`"))
        .ignore_then(just('s').labelled("`date:desc`"))
        .ignore_then(just('c').labelled("`date:desc`"))
        .to(SearchEmailsQueryOrder::Descending)
        .map(SearchEmailsQuerySorter::Date)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        just('f')
            .labelled("`from`")
            .ignore_then(just('r').labelled("`from`"))
            .ignore_then(just('o').labelled("`from`"))
            .ignore_then(just('m').labelled("`from`"))
            .to(SearchEmailsQueryOrder::Ascending)
            .map(SearchEmailsQuerySorter::From),
        from_asc(),
        from_desc(),
    ))
}

fn from_asc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('f')
        .labelled("`from:asc`")
        .ignore_then(just('r').labelled("`from:asc`"))
        .ignore_then(just('o').labelled("`from:asc`"))
        .ignore_then(just('m').labelled("`from:asc`"))
        .ignore_then(just(':').labelled("`from:asc`"))
        .ignore_then(just('a').labelled("`from:asc`"))
        .ignore_then(just('s').labelled("`from:asc`"))
        .ignore_then(just('c').labelled("`from:asc`"))
        .to(SearchEmailsQueryOrder::Ascending)
        .map(SearchEmailsQuerySorter::From)
}

fn from_desc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('f')
        .labelled("`from:desc`")
        .ignore_then(just('r').labelled("`from:desc`"))
        .ignore_then(just('o').labelled("`from:desc`"))
        .ignore_then(just('m').labelled("`from:desc`"))
        .ignore_then(just(':').labelled("`from:desc`"))
        .ignore_then(just('d').labelled("`from:desc`"))
        .ignore_then(just('e').labelled("`from:desc`"))
        .ignore_then(just('s').labelled("`from:desc`"))
        .ignore_then(just('c').labelled("`from:desc`"))
        .to(SearchEmailsQueryOrder::Descending)
        .map(SearchEmailsQuerySorter::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        just('t')
            .labelled("`to`")
            .ignore_then(just('o').labelled("`to`"))
            .to(SearchEmailsQueryOrder::Ascending)
            .map(SearchEmailsQuerySorter::To),
        to_asc(),
        to_desc(),
    ))
}

fn to_asc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to:asc`")
        .ignore_then(just('o').labelled("`to:asc`"))
        .ignore_then(just(':').labelled("`to:asc`"))
        .ignore_then(just('a').labelled("`to:asc`"))
        .ignore_then(just('s').labelled("`to:asc`"))
        .ignore_then(just('c').labelled("`to:asc`"))
        .to(SearchEmailsQueryOrder::Ascending)
        .map(SearchEmailsQuerySorter::To)
}

fn to_desc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to:desc`")
        .ignore_then(just('o').labelled("`to:desc`"))
        .ignore_then(just(':').labelled("`to:desc`"))
        .ignore_then(just('d').labelled("`to:desc`"))
        .ignore_then(just('e').labelled("`to:desc`"))
        .ignore_then(just('s').labelled("`to:desc`"))
        .ignore_then(just('c').labelled("`to:desc`"))
        .to(SearchEmailsQueryOrder::Descending)
        .map(SearchEmailsQuerySorter::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    choice((
        just('s')
            .labelled("`subject`")
            .ignore_then(just('u').labelled("`subject`"))
            .ignore_then(just('b').labelled("`subject`"))
            .ignore_then(just('j').labelled("`subject`"))
            .ignore_then(just('e').labelled("`subject`"))
            .ignore_then(just('c').labelled("`subject`"))
            .ignore_then(just('t').labelled("`subject`"))
            .to(SearchEmailsQueryOrder::Ascending)
            .map(SearchEmailsQuerySorter::Subject),
        subject_asc(),
        subject_desc(),
    ))
}

fn subject_asc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone {
    just('s')
        .labelled("`subject:asc`")
        .ignore_then(just('u').labelled("`subject:asc`"))
        .ignore_then(just('b').labelled("`subject:asc`"))
        .ignore_then(just('j').labelled("`subject:asc`"))
        .ignore_then(just('e').labelled("`subject:asc`"))
        .ignore_then(just('c').labelled("`subject:asc`"))
        .ignore_then(just('t').labelled("`subject:asc`"))
        .ignore_then(just(':').labelled("`subject:asc`"))
        .ignore_then(just('a').labelled("`subject:asc`"))
        .ignore_then(just('s').labelled("`subject:asc`"))
        .ignore_then(just('c').labelled("`subject:asc`"))
        .to(SearchEmailsQueryOrder::Ascending)
        .map(SearchEmailsQuerySorter::Subject)
}

fn subject_desc<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuerySorter, ParserError<'a>> + Clone
{
    just('s')
        .labelled("`subject:desc`")
        .ignore_then(just('u').labelled("`subject:desc`"))
        .ignore_then(just('b').labelled("`subject:desc`"))
        .ignore_then(just('j').labelled("`subject:desc`"))
        .ignore_then(just('e').labelled("`subject:desc`"))
        .ignore_then(just('c').labelled("`subject:desc`"))
        .ignore_then(just('t').labelled("`subject:desc`"))
        .ignore_then(just(':').labelled("`subject:desc`"))
        .ignore_then(just('d').labelled("`subject:desc`"))
        .ignore_then(just('e').labelled("`subject:desc`"))
        .ignore_then(just('s').labelled("`subject:desc`"))
        .ignore_then(just('c').labelled("`subject:desc`"))
        .to(SearchEmailsQueryOrder::Descending)
        .map(SearchEmailsQuerySorter::Subject)
}

fn space<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(' ')
}

pub(crate) fn order_by<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just("order by")
        .labelled("`order by` before sorters")
        .ignored()
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;

    use super::{SearchEmailsQueryOrder::*, SearchEmailsQuerySorter::*};

    #[test]
    fn empty_sorter() {
        assert_eq!(super::sorters().parse("").into_result(), Ok(vec![]));
    }

    #[test]
    fn simple_sorters() {
        assert_eq!(
            super::sorters().parse("date from to").into_result(),
            Ok(vec![Date(Ascending), From(Ascending), To(Ascending)])
        );
    }

    #[test]
    fn mixed_sorters() {
        assert_eq!(
            super::sorters()
                .parse("date:asc from subject:desc")
                .into_result(),
            Ok(vec![Date(Ascending), From(Ascending), Subject(Descending)])
        );
    }
}
