pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;

use async_trait::async_trait;
use chrono::{
    DateTime, Duration, Local, LocalResult, NaiveDate, NaiveDateTime, NaiveTime, ParseError,
};
use chumsky::prelude::*;
use thiserror::Error;

use crate::Result;

use super::Envelopes;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse date from list envelopes query")]
    ParseNaiveDateTimeError(#[source] ParseError),
    #[error("cannot parse date from list envelopes query: cannot apply local timezone to {0}")]
    ParseLocalDateTimeError(String),
    #[error("cannot parse date from list envelopes query: cannot choose between {0} and {1}")]
    ParseLocalDateTimeAmbiguousError(DateTime<Local>, DateTime<Local>),
}

#[async_trait]
pub trait ListEnvelopes: Send + Sync {
    /// List all available envelopes from the given folder matching
    /// the given pagination.
    async fn list_envelopes(&self, folder: &str, opts: ListEnvelopesOptions) -> Result<Envelopes>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ListEnvelopesOptions {
    pub page_size: usize,
    pub page: usize,
    pub filter: Option<ListEnvelopesFilter>,
    pub sort: Vec<ListEnvelopesComparator>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ListEnvelopesFilter {
    And(Box<ListEnvelopesFilter>, Box<ListEnvelopesFilter>),
    Or(Box<ListEnvelopesFilter>, Box<ListEnvelopesFilter>),
    Not(Box<ListEnvelopesFilter>),
    Folder(String),
    Before(DateTime<Local>),
    After(DateTime<Local>),
    From(String),
    To(String),
    Subject(String),
    Body(String),
    Keyword(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ListEnvelopesComparator {
    Descending(Box<ListEnvelopesComparator>),
    SentAt,
    ReceivedAt,
    From,
    To,
    Subject,
}

pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

pub(crate) fn filter<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone
{
    recursive(|filter| {
        let filter = choice((
            filter.delimited_by(just(LPARENS), just(RPARENS)),
            condition(),
        ))
        .padded();

        let not = just("not").padded().repeated().foldr(filter, |_, filter| {
            ListEnvelopesFilter::Not(Box::new(filter))
        });

        let and = not.clone().foldl(
            just("and").padded().ignored().then(not).repeated(),
            |left, (_, right)| ListEnvelopesFilter::And(Box::new(left), Box::new(right)),
        );

        let or = and.clone().foldl(
            just("or").padded().ignored().then(and).repeated(),
            |left, (_, right)| ListEnvelopesFilter::Or(Box::new(left), Box::new(right)),
        );

        or
    })
}

fn condition<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    choice((
        from(),
        to(),
        subject(),
        body(),
        keyword(),
        before(),
        after(),
    ))
}

fn before<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("before")
        .padded()
        .ignore_then(date(|dt| dt))
        .map(ListEnvelopesFilter::Before)
}

fn after<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("after")
        .padded()
        .ignore_then(date(|dt| dt + Duration::days(1)))
        .map(ListEnvelopesFilter::After)
}

fn from<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("from")
        .padded()
        .ignore_then(val())
        .map(ListEnvelopesFilter::From)
}

fn to<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("to")
        .padded()
        .ignore_then(val())
        .map(ListEnvelopesFilter::To)
}

fn subject<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("subject")
        .padded()
        .ignore_then(val())
        .map(ListEnvelopesFilter::Subject)
}

fn body<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("body")
        .padded()
        .ignore_then(val())
        .map(ListEnvelopesFilter::Body)
}

fn keyword<'a>() -> impl Parser<'a, &'a str, ListEnvelopesFilter, ParserError<'a>> + Clone {
    just("keyword")
        .padded()
        .ignore_then(val())
        .map(ListEnvelopesFilter::Keyword)
}

const SPACE: char = ' ';
const LPARENS: char = '(';
const RPARENS: char = ')';

const BACKSLASH: char = '\\';
fn backslash<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(BACKSLASH).labelled("backslash")
}

const DOUBLE_QUOTE: char = '"';
fn dquote<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
    just(DOUBLE_QUOTE).labelled("double quote")
}

fn val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    choice((quoted_val(), unquoted_val()))
}

/// The quoted property value parser.
///
/// It parses all characters except the backslack and the double quote
/// characters. They still can be parsed by escaping them with a
/// backslack.
fn quoted_val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, DOUBLE_QUOTE];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .collect()
    .delimited_by(dquote().ignored(), dquote().ignored())
}

/// The property value parser.
///
/// It parses all characters except the backslack, the space and the
/// greater-than characters. They still can be parsed by escaping them
/// with a backslash.
fn unquoted_val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, SPACE, LPARENS, RPARENS];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .at_least(1)
    .collect()
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

#[cfg(test)]
mod tests {
    use chrono::{Local, TimeZone};
    use chumsky::prelude::*;

    use super::ListEnvelopesFilter;

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
            Ok(ListEnvelopesFilter::Before(
                Local.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap()
            ))
        );
    }

    #[test]
    fn after() {
        assert_eq!(
            super::after().parse("after 2024-01-01").into_result(),
            Ok(ListEnvelopesFilter::After(
                Local.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap()
            ))
        );
    }

    #[test]
    fn from() {
        assert_eq!(
            super::from().parse("from unquoted-val").into_result(),
            Ok(ListEnvelopesFilter::From("unquoted-val".into())),
        );

        assert_eq!(
            super::from().parse("from \"quoted val\"").into_result(),
            Ok(ListEnvelopesFilter::From("quoted val".into())),
        );
    }

    #[test]
    fn filter() {
        use ListEnvelopesFilter::*;

        assert_eq!(
            super::filter()
                .parse("from f and to t and subject s")
                .into_result(),
            Ok(And(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::filter()
                .parse("from f and (to t and subject s)")
                .into_result(),
            Ok(And(
                Box::new(From("f".into())),
                Box::new(And(Box::new(To("t".into())), Box::new(Subject("s".into())))),
            )),
        );

        assert_eq!(
            super::filter()
                .parse("from f and to t or subject s")
                .into_result(),
            Ok(Or(
                Box::new(And(Box::new(From("f".into())), Box::new(To("t".into())))),
                Box::new(Subject("s".into()))
            )),
        );

        assert_eq!(
            super::filter()
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
            super::filter()
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
