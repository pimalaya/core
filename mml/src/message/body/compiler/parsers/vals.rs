use crate::message::body::{compiler::tokens::Val, BACKSLASH, DOUBLE_QUOTE, GREATER_THAN, SPACE};

use super::prelude::*;

/// Represents the property value parser. It parses all characters
/// except the backslack, the space and the greater-than
/// characters. They still can be parsed by prefixing them with a
/// backslack (escaping).
pub(crate) fn val<'a>() -> impl Parser<'a, &'a str, Val, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, SPACE, GREATER_THAN];

    choice((
        just(BACKSLASH)
            .labelled("escaped character")
            .ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .at_least(1)
    .collect()
}

/// Represents the quoted property value parser. It parses all
/// characters except the backslack and the double quote
/// characters. They still can be parsed by prefixing them with a
/// backslack (escaping).
pub(crate) fn quoted_val<'a>() -> impl Parser<'a, &'a str, Val, ParserError<'a>> + Clone {
    let escapable_chars = [BACKSLASH, DOUBLE_QUOTE];

    choice((
        just(BACKSLASH)
            .labelled("escaped character")
            .ignore_then(one_of(escapable_chars)),
        none_of(escapable_chars),
    ))
    .repeated()
    .collect::<String>()
    .delimited_by(just(DOUBLE_QUOTE), just(DOUBLE_QUOTE))
}

/// Represents the parser that can parse either the given string or
/// the quoted version of it.
pub(crate) fn maybe_quoted_val<'a>(
    key: impl ToString,
) -> impl Parser<'a, &'a str, Val, ParserError<'a>> + Clone {
    choice((
        just(key.to_string()),
        just(key.to_string())
            .delimited_by(just(DOUBLE_QUOTE), just(DOUBLE_QUOTE))
            .labelled("quoted value"),
    ))
}

#[cfg(test)]
mod vals {
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
        assert_eq!(
            super::quoted_val().parse("\"\"").into_result(),
            Ok("".into())
        );

        assert_eq!(
            super::quoted_val().parse("\"quoted val\"").into_result(),
            Ok("quoted val".into()),
        );

        assert_eq!(
            super::quoted_val()
                .parse("\"\\\\quoted \\\"val\\\"\"")
                .into_result(),
            Ok("\\quoted \"val\"".into()),
        );
    }

    #[test]
    fn maybe_quoted_val() {
        assert_eq!(
            super::maybe_quoted_val("key").parse("key").into_result(),
            Ok("key".into())
        );
        assert_eq!(
            super::maybe_quoted_val("key")
                .parse("\"key\"")
                .into_result(),
            Ok("key".into())
        );
    }
}
