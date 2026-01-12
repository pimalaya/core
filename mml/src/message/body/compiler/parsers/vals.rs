//! # Property value parsers
//!
//! This module contains all property value parsers needed to parse
//! MML message bodies: [val], [quoted_val] and
//! [maybe_quoted_const_val].

use crate::message::body::{compiler::tokens::Val, BACKSLASH, DOUBLE_QUOTE, GREATER_THAN, SPACE};

use super::prelude::*;

/// The property value parser.
///
/// It parses all characters except the backslack, the space and the
/// greater-than characters. They still can be parsed by escaping them
/// with a backslash.
pub(crate) fn val<'a>() -> impl Parser<'a, &'a str, String, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, SPACE, GREATER_THAN];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .at_least(1)
    .collect()
}

/// The quoted property value parser.
///
/// It parses all characters except the backslack and the double quote
/// characters. They still can be parsed by escaping them with a
/// backslack.
pub(crate) fn quoted_val<'a>() -> impl Parser<'a, &'a str, Val<'a>, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, DOUBLE_QUOTE];

    choice((
        backslash().ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .to_slice()
    .delimited_by(dquote(), dquote())
}

/// The maybe quoted const property value parser.
///
/// It parses either the given const value or the quoted version of
/// it.
pub(crate) fn maybe_quoted_const_val(
    val: &str,
) -> impl Parser<'_, &str, Val<'_>, ParserError<'_>> + Clone {
    choice((
        just(val).to_slice().delimited_by(dquote(), dquote()),
        just(val).to_slice(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn val() {
        assert_eq!(
            super::val().parse("value").into_result(),
            Ok("value".into())
        );

        assert_eq!(
            super::val().parse("escaped\\ space\"").into_result(),
            Ok("escaped space\"".into()),
        );

        // example from the Emacs MML module:
        assert_eq!(
            super::val().parse("/home/user/#hello$^yes").into_result(),
            Ok("/home/user/#hello$^yes".into()),
        );
    }

    #[test]
    fn quoted_val() {
        assert_eq!(super::quoted_val().parse("\"\"").into_result(), Ok(""));

        assert_eq!(
            super::quoted_val().parse("\"quoted val\"").into_result(),
            Ok("quoted val"),
        );

        assert_eq!(
            super::quoted_val()
                .parse("\"\\\\quoted \\\"val\\\"\"")
                .into_result(),
            Ok("\\\\quoted \\\"val\\\""),
        );
    }

    #[test]
    fn maybe_quoted_val() {
        assert_eq!(
            super::maybe_quoted_const_val("key")
                .parse("key")
                .into_result(),
            Ok("key")
        );

        assert_eq!(
            super::maybe_quoted_const_val("key")
                .parse("\"key\"")
                .into_result(),
            Ok("key")
        );
    }
}
