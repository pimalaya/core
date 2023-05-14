use crate::lexer::tpl::Val;

use super::{prelude::*, BACKSLASH, DOUBLE_QUOTE, GREATER_THAN, SPACE};

/// Represents the property value parser. It parses all characters
/// except the backslack, the space and the greater-than
/// characters. They still can be parsed by prefixing them with a
/// backslack (escaping).
pub(crate) fn val() -> impl Parser<char, Val, Error = Simple<char>> {
    let escapable_chars = [BACKSLASH, SPACE, GREATER_THAN];

    choice((
        none_of(escapable_chars),
        just(BACKSLASH).ignore_then(one_of(escapable_chars)),
    ))
    .repeated()
    .at_least(1)
    .collect()
}

/// Represents the quoted property value parser. It parses all
/// characters except the backslack and the double quote
/// characters. They still can be parsed by prefixing them with a
/// backslack (escaping).
pub(crate) fn quoted_val() -> impl Parser<char, Val, Error = Simple<char>> {
    let escapable_chars = [BACKSLASH, DOUBLE_QUOTE];

    choice((
        none_of(escapable_chars),
        just(BACKSLASH).ignore_then(one_of(escapable_chars)),
    ))
    .repeated()
    .delimited_by(just(DOUBLE_QUOTE), just(DOUBLE_QUOTE))
    .collect()
}

/// Represents the parser that can parse either the given string or
/// the quoted version of it.
pub(crate) fn maybe_quoted_val<T: ToString>(
    key: T,
) -> impl Parser<char, String, Error = Simple<char>> {
    choice((
        just(key.to_string()),
        just(key.to_string()).delimited_by(just(DOUBLE_QUOTE), just(DOUBLE_QUOTE)),
    ))
}

#[cfg(test)]
mod vals {
    use super::*;

    #[test]
    fn val() {
        assert_eq!(super::val().parse("value"), Ok("value".into()));
        assert_eq!(super::val().parse("value ignored"), Ok("value".into()));
        assert_eq!(
            super::val().parse("escaped\\ space\" ignored"),
            Ok("escaped space\"".into()),
        );

        // example from the Emacs MML module:
        assert_eq!(
            super::val().parse("/home/user/#hello$^yes"),
            Ok("/home/user/#hello$^yes".into()),
        );
    }

    #[test]
    fn quoted_val() {
        assert_eq!(super::quoted_val().parse("\"\""), Ok("".into()));

        assert_eq!(
            super::quoted_val().parse("\"quoted val\" ignored"),
            Ok("quoted val".into()),
        );

        assert_eq!(
            super::quoted_val().parse("\"\\\\quoted \\\"val\\\"\" ignored"),
            Ok("\\quoted \"val\"".into()),
        );
    }

    #[test]
    fn maybe_quoted_val() {
        assert_eq!(
            super::maybe_quoted_val("key").parse("key"),
            Ok("key".into())
        );
        assert_eq!(
            super::maybe_quoted_val("key").parse("\"key\""),
            Ok("key".into())
        );
    }
}
