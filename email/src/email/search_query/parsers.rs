use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, ParseError,
};
use chumsky::prelude::*;
use thiserror::Error;

use super::SearchEmailsQuery;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse date from list envelopes query")]
    ParseNaiveDateTimeError(#[source] ParseError),
    #[error("cannot parse date from list envelopes query: cannot apply local timezone to {0}")]
    ParseLocalDateTimeError(String),
    #[error("cannot parse date from list envelopes query: cannot choose between {0} and {1}")]
    ParseLocalDateTimeAmbiguousError(DateTime<Local>, DateTime<Local>),
}

const SPACE: char = ' ';
const LPAREN: char = '(';
const RPAREN: char = ')';
const BSLASH: char = '\\';
const DQUOTE: char = '"';

type ParserError<'a> = extra::Err<Rich<'a, char>>;

pub(crate) fn query<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    recursive(|filter| {
        let space_or_end = choice((
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space between filters"),
            just(RPAREN).or_not().ignored().rewind(),
            end(),
        ));

        let simple_filter = choice((
            before(),
            after(),
            from(),
            to(),
            subject(),
            body(),
            keyword(),
        ));

        let nested_filter = filter.delimited_by(
            just(LPAREN).labelled("beginning of nested (filter)"),
            just(RPAREN).labelled("ending of nested (filter)"),
        );

        let filter = choice((
            nested_filter.labelled("nested (filter)"),
            simple_filter.labelled("filter"),
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
        .labelled("not")
        .ignore_then(just('o').labelled("o in not"))
        .ignore_then(just('t').labelled("t in not"))
        .ignore_then(just(' ').repeated().at_least(1).labelled("space after not"))
}

fn and<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('a')
        .labelled("and")
        .ignore_then(just('n').labelled("n in and"))
        .ignore_then(just('d').labelled("d in and"))
        .ignore_then(just(' ').repeated().at_least(1).labelled("space after and"))
}

fn or<'a>() -> impl Parser<'a, &'a str, (), ParserError<'a>> + Clone {
    just('o')
        .labelled("or")
        .ignore_then(just('r').labelled("r in or"))
        .ignore_then(just(' ').repeated().at_least(1).labelled("space after or"))
}

fn before<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('b')
        .labelled("before")
        .ignore_then(just('e').labelled("e in before"))
        .ignore_then(just('f').labelled("f in before"))
        .ignore_then(just('o').labelled("o in before"))
        .ignore_then(just('r').labelled("r in before"))
        .ignore_then(just('e').labelled("e in before"))
        .ignore_then(
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space after before"),
        )
        .ignore_then(date(|dt| dt).labelled("before date filter"))
        .map(SearchEmailsQuery::Before)
}

fn after<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('a')
        .labelled("after")
        .ignore_then(just('f').labelled("f in after"))
        .ignore_then(just('t').labelled("t in after"))
        .ignore_then(just('e').labelled("e in after"))
        .ignore_then(just('r').labelled("r in after"))
        .ignore_then(
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space after after"),
        )
        .ignore_then(date(|dt| dt + Duration::days(1)).labelled("after date filter"))
        .map(SearchEmailsQuery::After)
}

fn from<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('f')
        .labelled("from")
        .ignore_then(just('r').labelled("r in from"))
        .ignore_then(just('o').labelled("o in from"))
        .ignore_then(just('m').labelled("m in from"))
        .ignore_then(
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space after from"),
        )
        .ignore_then(val().labelled("from filter"))
        .map(SearchEmailsQuery::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('t')
        .labelled("to")
        .ignore_then(just('o').labelled("o in to"))
        .ignore_then(just(' ').repeated().at_least(1).labelled("space after to"))
        .ignore_then(val().labelled("to filter"))
        .map(SearchEmailsQuery::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('s')
        .labelled("subject")
        .ignore_then(just('u').labelled("u in subject"))
        .ignore_then(just('b').labelled("b in subject"))
        .ignore_then(just('j').labelled("j in subject"))
        .ignore_then(just('e').labelled("e in subject"))
        .ignore_then(just('c').labelled("c in subject"))
        .ignore_then(just('t').labelled("t in subject"))
        .ignore_then(
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space after subject"),
        )
        .ignore_then(val().labelled("subject filter"))
        .map(SearchEmailsQuery::Subject)
}

fn body<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('b')
        .labelled("body")
        .ignore_then(just('o').labelled("o in body"))
        .ignore_then(just('d').labelled("d in body"))
        .ignore_then(just('y').labelled("y in body"))
        .ignore_then(
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space after body"),
        )
        .ignore_then(val().labelled("body filter"))
        .map(SearchEmailsQuery::Body)
}

fn keyword<'a>() -> impl Parser<'a, &'a str, SearchEmailsQuery, ParserError<'a>> + Clone {
    just('k')
        .labelled("keyword")
        .ignore_then(just('e').labelled("e in keyword"))
        .ignore_then(just('y').labelled("y in keyword"))
        .ignore_then(just('w').labelled("w in keyword"))
        .ignore_then(just('o').labelled("o in keyword"))
        .ignore_then(just('r').labelled("r in keyword"))
        .ignore_then(just('d').labelled("d in keyword"))
        .ignore_then(
            just(' ')
                .repeated()
                .at_least(1)
                .labelled("space after keyword"),
        )
        .ignore_then(val().labelled("keyword filter"))
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
    let escapable_chars = [BSLASH, DQUOTE];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .collect()
    .delimited_by(dquote().ignored(), dquote().ignored())
}

fn unquoted_val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = [BSLASH, SPACE, LPAREN, RPAREN];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .at_least(1)
    .collect()
}

fn backslash<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(BSLASH).labelled("backslash")
}

fn dquote<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(DQUOTE).labelled("double quote")
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
