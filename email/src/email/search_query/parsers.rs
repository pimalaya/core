//! # Search emails query parsers
//!
//! This module contains parsers needed to parse a full search emails
//! query, and exposes a [`query`] parser. Parsing is based on the
//! great lib [chumsky].

use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, ParseError,
};
use chumsky::prelude::*;
use thiserror::Error;

use super::SearchEmailsQuery;

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

type ParserError<'a> = extra::Err<Rich<'a, char>>;

pub(crate) fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
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
            before(),
            after(),
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

        let not = not()
            .repeated()
            .foldr(filter, |_, filter| SearchEmailsQuery::Not(Box::new(filter)));

        let and = not
            .clone()
            .foldl(and().then(not).repeated(), |left, (_, right)| {
                SearchEmailsQuery::And(Box::new(left), Box::new(right))
            });

        let or = and
            .clone()
            .foldl(or().then(and).repeated(), |left, (_, right)| {
                SearchEmailsQuery::Or(Box::new(left), Box::new(right))
            });

        or
    })
}

fn not<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('n')
        .labelled("`not`")
        .ignore_then(just('o').labelled("o of `not`"))
        .ignore_then(just('t').labelled("t of `not`"))
        .ignore_then(space().labelled("space after `not`").repeated().at_least(1))
}

fn and<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('a')
        .labelled("`and`")
        .ignore_then(just('n').labelled("n of `and`"))
        .ignore_then(just('d').labelled("d of `and`"))
        .ignore_then(space().labelled("space after `and`").repeated().at_least(1))
}

fn or<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('o')
        .labelled("`or`")
        .ignore_then(just('r').labelled("r of `or`"))
        .ignore_then(space().labelled("space after `or`").repeated().at_least(1))
}

fn before<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('b')
        .labelled("`before`")
        .ignore_then(just('e').labelled("e of `before`"))
        .ignore_then(just('f').labelled("f of `before`"))
        .ignore_then(just('o').labelled("o of `before`"))
        .ignore_then(just('r').labelled("r of `before`"))
        .ignore_then(just('e').labelled("e of `before`"))
        .ignore_then(
            space()
                .labelled("space after `before`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(date(|dt| dt).labelled("value after `before`"))
        .map(SearchEmailsQuery::Before)
}

fn after<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('a')
        .labelled("`after`")
        .ignore_then(just('f').labelled("f of `after`"))
        .ignore_then(just('t').labelled("t of `after`"))
        .ignore_then(just('e').labelled("e of `after`"))
        .ignore_then(just('r').labelled("r of `after`"))
        .ignore_then(
            space()
                .labelled("space after `after`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(date(|dt| dt + Duration::days(1)).labelled("value after `after`"))
        .map(SearchEmailsQuery::After)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('f')
        .labelled("`from`")
        .ignore_then(just('r').labelled("r of `from`"))
        .ignore_then(just('o').labelled("o of `from`"))
        .ignore_then(just('m').labelled("m of `from`"))
        .ignore_then(
            space()
                .repeated()
                .at_least(1)
                .labelled("space after `from`"),
        )
        .ignore_then(val().labelled("value after `from`"))
        .map(SearchEmailsQuery::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('t')
        .labelled("`to`")
        .ignore_then(just('o').labelled("o of `to`"))
        .ignore_then(space().labelled("space after `to`").repeated().at_least(1))
        .ignore_then(val().labelled("value after `to`"))
        .map(SearchEmailsQuery::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('s')
        .labelled("`subject`")
        .ignore_then(just('u').labelled("u of `subject`"))
        .ignore_then(just('b').labelled("b of `subject`"))
        .ignore_then(just('j').labelled("j of `subject`"))
        .ignore_then(just('e').labelled("e of `subject`"))
        .ignore_then(just('c').labelled("c of `subject`"))
        .ignore_then(just('t').labelled("t of `subject`"))
        .ignore_then(
            space()
                .labelled("space after `subject`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(val().labelled("value after `subject`"))
        .map(SearchEmailsQuery::Subject)
}

fn body<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('b')
        .labelled("`body`")
        .ignore_then(just('o').labelled("o of `body`"))
        .ignore_then(just('d').labelled("d of `body`"))
        .ignore_then(just('y').labelled("y of `body`"))
        .ignore_then(
            space()
                .labelled("space after `body`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(val().labelled("value after `body`"))
        .map(SearchEmailsQuery::Body)
}

fn keyword<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('k')
        .labelled("`keyword`")
        .ignore_then(just('e').labelled("e of `keyword`"))
        .ignore_then(just('y').labelled("y of `keyword`"))
        .ignore_then(just('w').labelled("w of `keyword`"))
        .ignore_then(just('o').labelled("o of `keyword`"))
        .ignore_then(just('r').labelled("r of `keyword`"))
        .ignore_then(just('d').labelled("d of `keyword`"))
        .ignore_then(
            space()
                .labelled("space after `keyword`")
                .repeated()
                .at_least(1),
        )
        .ignore_then(val().labelled("value after `keyword`"))
        .map(SearchEmailsQuery::Keyword)
}

fn date<'a>(
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
    val().try_map(move |ref s, span| {
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

fn val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    choice((quoted_val(), unquoted_val()))
}

fn quoted_val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = ['\\', '"'];

    choice((
        bslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .collect()
    .delimited_by(dquote().ignored(), dquote().ignored())
}

fn unquoted_val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
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

    use super::SearchEmailsQuery::*;

    #[test]
    fn unquoted_val() {
        assert_eq!(
            super::unquoted_val().parse("value").into_result(),
            Ok("value".into())
        );

        assert_eq!(
            super::unquoted_val().parse("escaped\\ space").into_result(),
            Ok("escaped space".into()),
        );
    }

    #[test]
    fn quoted_val() {
        assert_eq!(
            super::quoted_val().parse("\"\"").into_result(),
            Ok("".into())
        );

        assert_eq!(
            super::quoted_val().parse("\"quoted val\"").into_result(),
            Ok("quoted val".into()),
        );
    }

    #[test]
    fn val() {
        assert_eq!(
            super::val().parse("unquoted-val").into_result(),
            Ok("unquoted-val".into())
        );

        assert_eq!(
            super::val().parse("\"quoted val\"").into_result(),
            Ok("quoted val".into()),
        );
    }

    #[test]
    fn before() {
        assert_eq!(
            super::before().parse("before 2024-01-01").into_result(),
            Ok(Before(Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()))
        );
    }

    #[test]
    fn after() {
        assert_eq!(
            super::after().parse("after 2024-01-01").into_result(),
            Ok(After(Local.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()))
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
            Ok(From("quoted val".into())),
        );
    }

    #[test]
    fn filter() {
        assert_eq!(
            super::query()
                .parse("from f and to t and subject s")
                .into_result(),
            Ok(And(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::query()
                .parse("from f and (to t and subject s)")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(And(Box::new(To("t".into())), Box::new(Subject("s".into())))),
            )),
        );

        assert_eq!(
            super::query()
                .parse("from f and to t or subject s")
                .into_result(),
            Ok(Or(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::query()
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
            super::query()
                .parse("from f and (to t or subject \"s with parens )\")")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(Or(
                    Box::new(To("t".into())),
                    Box::new(Subject("s with parens )".into()))
                )),
            )),
        );
    }
}
