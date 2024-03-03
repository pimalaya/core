//! # Search emails query filters parser
//!
//! This module contains parsers needed to parse a full search emails
//! query, and exposes a [`query`] parser. Parsing is based on the
//! great lib [`chumsky`].

use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, ParseError,
};
use chumsky::prelude::*;
use thiserror::Error;

use crate::search_query::parser::ParserError;

use super::SearchEmailsQueryFilter;

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

pub(crate) fn filters<'a>(
) -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    recursive(|filter| {
        let space_or_end = choice((
            space()
                .labelled("space between filters")
                .repeated()
                .at_least(1),
            rparen().or_not().ignored().rewind(),
            end(),
        ));

        let filter = choice((
            date(),
            before_date(),
            after_date(),
            from(),
            to(),
            subject(),
            body(),
            keyword(),
            filter
                .delimited_by(lparen(), rparen())
                .labelled("(nested filter)"),
        ))
        .then_ignore(space_or_end);

        let not = not().repeated().foldr(filter, |_, filter| {
            SearchEmailsQueryFilter::Not(Box::new(filter))
        });

        let and = not
            .clone()
            .foldl(and().then(not).repeated(), |left, (_, right)| {
                SearchEmailsQueryFilter::And(Box::new(left), Box::new(right))
            });

        let or = and
            .clone()
            .foldl(or().then(and).repeated(), |left, (_, right)| {
                SearchEmailsQueryFilter::Or(Box::new(left), Box::new(right))
            });

        or
    })
}

fn not<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('n')
        .labelled("`not`")
        .ignore_then(just('o').labelled("`not`"))
        .ignore_then(just('t').labelled("`not`"))
        .ignore_then(space().labelled("space after `not`").repeated().at_least(1))
}

fn and<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('a')
        .labelled("`and`")
        .ignore_then(just('n').labelled("`and`"))
        .ignore_then(just('d').labelled("`and`"))
        .ignore_then(space().labelled("space after `and`").repeated().at_least(1))
}

fn or<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('o')
        .labelled("`or`")
        .ignore_then(just('r').labelled("`or`"))
        .ignore_then(space().labelled("space after `or`").repeated().at_least(1))
}

fn date<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('d')
        .labelled("`date`")
        .ignore_then(just('a').labelled("`date`"))
        .ignore_then(just('t').labelled("`date`"))
        .ignore_then(just('e').labelled("`date`"))
        .ignore_then(
            space()
                .labelled("space after `date`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(date_fmt(|dt| dt).labelled("date format after `date`"))
        .map(SearchEmailsQueryFilter::Date)
}

fn before_date<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('b')
        .labelled("`before`")
        .ignore_then(just('e').labelled("`before`"))
        .ignore_then(just('f').labelled("`before`"))
        .ignore_then(just('o').labelled("`before`"))
        .ignore_then(just('r').labelled("`before`"))
        .ignore_then(just('e').labelled("`before`"))
        .ignore_then(
            space()
                .labelled("space after `before`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(date_fmt(|dt| dt - Duration::days(1)).labelled("pattern after `before`"))
        .map(SearchEmailsQueryFilter::BeforeDate)
}

fn after_date<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('a')
        .labelled("`after`")
        .ignore_then(just('f').labelled("`after`"))
        .ignore_then(just('t').labelled("`after`"))
        .ignore_then(just('e').labelled("`after`"))
        .ignore_then(just('r').labelled("`after`"))
        .ignore_then(
            space()
                .labelled("space after `after`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(date_fmt(|dt| dt + Duration::days(1)).labelled("pattern after `after`"))
        .map(SearchEmailsQueryFilter::AfterDate)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('f')
        .labelled("`from`")
        .ignore_then(just('r').labelled("`from`"))
        .ignore_then(just('o').labelled("`from`"))
        .ignore_then(just('m').labelled("`from`"))
        .ignore_then(
            space()
                .labelled("space after `from`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `from`"))
        .map(SearchEmailsQueryFilter::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to`")
        .ignore_then(just('o').labelled("`to`"))
        .ignore_then(space().labelled("space after `to`").repeated().at_least(1))
        .ignore_then(pattern().labelled("pattern after `to`"))
        .map(SearchEmailsQueryFilter::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('s')
        .labelled("`subject`")
        .ignore_then(just('u').labelled("`subject`"))
        .ignore_then(just('b').labelled("`subject`"))
        .ignore_then(just('j').labelled("`subject`"))
        .ignore_then(just('e').labelled("`subject`"))
        .ignore_then(just('c').labelled("`subject`"))
        .ignore_then(just('t').labelled("`subject`"))
        .ignore_then(
            space()
                .labelled("space after `subject`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `subject`"))
        .map(SearchEmailsQueryFilter::Subject)
}

fn body<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('b')
        .labelled("`body`")
        .ignore_then(just('o').labelled("`body`"))
        .ignore_then(just('d').labelled("`body`"))
        .ignore_then(just('y').labelled("`body`"))
        .ignore_then(
            space()
                .labelled("space after `body`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `body`"))
        .map(SearchEmailsQueryFilter::Body)
}

fn keyword<'a>() -> impl Parser<'a, &'a str, SearchEmailsQueryFilter, ParserError<'a>> + Clone {
    just('k')
        .labelled("`keyword`")
        .ignore_then(just('e').labelled("`keyword`"))
        .ignore_then(just('y').labelled("`keyword`"))
        .ignore_then(just('w').labelled("`keyword`"))
        .ignore_then(just('o').labelled("`keyword`"))
        .ignore_then(just('r').labelled("`keyword`"))
        .ignore_then(just('d').labelled("`keyword`"))
        .ignore_then(
            space()
                .labelled("space after `keyword`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(pattern().labelled("pattern after `keyword`"))
        .map(SearchEmailsQueryFilter::Keyword)
}

fn date_fmt<'a>(
    cb: impl Fn(NaiveDateTime) -> NaiveDateTime + Clone + 'a,
) -> impl Parser<'a, &'a str, DateTime<Local>, ParserError<'a>> + Clone {
    choice((
        date_with_fmt("%Y-%m-%d", cb.clone()),
        date_with_fmt("%Y/%m/%d", cb.clone()),
        date_with_fmt("%d-%m-%Y", cb.clone()),
        date_with_fmt("%d/%m/%Y", cb.clone()),
    ))
}

fn date_with_fmt<'a>(
    fmt: &'a str,
    cb: impl Fn(NaiveDateTime) -> NaiveDateTime + Clone + 'a,
) -> impl Parser<'a, &'a str, DateTime<Local>, ParserError<'a>> + Clone {
    pattern().try_map(move |ref s, span| {
        let dt = NaiveDate::parse_from_str(s, fmt)
            .map(|d| d.and_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap()))
            .map(|dt| cb(dt))
            .map(|dt| dt.and_local_timezone(Local));

        let dt = match dt {
            Err(err) => Err(Error::ParseNaiveDateTimeError(err)),
            Ok(LocalResult::None) => Err(Error::ParseLocalDateTimeError(s.clone())),
            Ok(LocalResult::Single(dt)) => Ok(dt),
            Ok(LocalResult::Ambiguous(dt1, dt2)) => {
                Err(Error::ParseLocalDateTimeAmbiguousError(dt1, dt2))
            }
        };

        dt.map_err(|err| Rich::custom(span, err))
    })
}

fn pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    choice((quoted_pattern(), unquoted_pattern()))
}

fn quoted_pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', '"'];

    dquote()
        .then(
            choice((
                bslash().ignore_then(one_of(escapable_chars)),
                none_of(escapable_chars),
            ))
            .repeated(),
        )
        .then(dquote())
        .to_slice()
        .map(String::from)
}

fn unquoted_pattern<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', ' ', '(', ')'];

    choice((
        bslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .at_least(1)
    .collect()
}

fn space<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(' ')
}

fn lparen<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just('(').labelled("nested filter opening '('")
}

fn rparen<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(')').labelled("nested filter closing ')'")
}

fn bslash<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just('\\').labelled("backslash")
}

fn dquote<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just('"').labelled("double quote")
}

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};
    use chumsky::prelude::*;

    use super::SearchEmailsQueryFilter::*;

    #[test]
    fn pattern() {
        assert_eq!(
            super::unquoted_pattern().parse("pattern").into_result(),
            Ok("pattern".into())
        );

        assert_eq!(
            super::unquoted_pattern()
                .parse("escaped\\ chars\\)")
                .into_result(),
            Ok("escaped chars)".into()),
        );

        assert_eq!(
            super::quoted_pattern().parse("\"\"").into_result(),
            Ok("\"\"".into())
        );

        assert_eq!(
            super::quoted_pattern()
                .parse("\"quoted pattern\"")
                .into_result(),
            Ok("\"quoted pattern\"".into()),
        );
    }

    #[test]
    fn before_date() {
        assert_eq!(
            super::before_date()
                .parse("before 2024-01-01")
                .into_result(),
            Ok(BeforeDate(
                Local.with_ymd_and_hms(2023, 12, 31, 0, 0, 0).unwrap()
            ))
        );
    }

    #[test]
    fn after_date() {
        assert_eq!(
            super::after_date().parse("after 2024-01-01").into_result(),
            Ok(AfterDate(
                Local.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()
            ))
        );
    }

    #[test]
    fn from() {
        assert_eq!(
            super::from().parse("from unquoted-val").into_result(),
            Ok(From("unquoted-val".into())),
        );

        assert_eq!(
            super::from().parse("from \"quoted val\"").into_result(),
            Ok(From("\"quoted val\"".into())),
        );
    }

    #[test]
    fn filter() {
        assert_eq!(
            super::filters()
                .parse("from f and to t and subject s")
                .into_result(),
            Ok(And(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::filters()
                .parse("subject or or subject and")
                .into_result(),
            Ok(Or(
                Box::new(Subject("or".into())),
                Box::new(Subject("and".into()))
            )),
        );

        assert_eq!(
            super::filters()
                .parse("from f and (to t and subject s)")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(And(Box::new(To("t".into())), Box::new(Subject("s".into())))),
            )),
        );

        assert_eq!(
            super::filters()
                .parse("from f and to t or subject s")
                .into_result(),
            Ok(Or(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::filters()
                .parse("from f or to t and not subject s")
                .into_result(),
            Ok(Or(
                Box::new(From("f".into())),
                Box::new(And(
                    Box::new(To("t".into())),
                    Box::new(Not(Box::new(Subject("s".into()))))
                )),
            )),
        );

        assert_eq!(
            super::filters()
                .parse("from f and (to t or subject \"s with parens )\")")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(Or(
                    Box::new(To("t".into())),
                    Box::new(Subject("\"s with parens )\"".into()))
                )),
            )),
        );
    }
}
