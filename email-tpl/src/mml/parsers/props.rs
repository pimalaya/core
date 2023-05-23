use crate::mml::tokens::{Prop, DISPOSITION, ENCRYPT, FILENAME, NAME, SIGN, TYPE};

use super::{maybe_quoted_val, prelude::*, quoted_val, val};

/// Represents the multipart type property parser. It parses value for
/// the `Content-Type` of the multipart. The value can be `mixed`,
/// `alternative` or `related`.
pub(crate) fn multipart_type() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(TYPE)
        .then_ignore(just('=').padded())
        .then(choice((
            maybe_quoted_val("mixed"),
            maybe_quoted_val("alternative"),
            maybe_quoted_val("related"),
        )))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the part type property parser. It parses value for the
/// `Content-Type` header of the part.
pub(crate) fn part_type() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(TYPE)
        .then_ignore(just('=').padded())
        .then(choice((val(), quoted_val())))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the name property parser.
pub(crate) fn name() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(NAME)
        .then_ignore(just('=').padded())
        .then(choice((val(), quoted_val())))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the disposition property parser. It parses value for
/// the `Content-Disposition` header of the part. The value can be
/// `inline` or `attachment`.
pub(crate) fn disposition() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(DISPOSITION)
        .then_ignore(just('=').padded())
        .then(choice((
            maybe_quoted_val("inline"),
            maybe_quoted_val("attachment"),
        )))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the filename property parser.
pub(crate) fn filename() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(FILENAME)
        .then_ignore(just('=').padded())
        .then(choice((val(), quoted_val())))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the encrypt property parser. The value can only be
/// `command` for now, other value will be implemented in the future
/// like `pgp` or `smime`. The command refers to
/// [`CompilerOpts::pgp_encrypt_cmd`].
pub(crate) fn encrypt() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(ENCRYPT)
        .then_ignore(just('=').padded())
        .then(maybe_quoted_val("command"))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the sign property parser. The value can only be
/// `command` for now, other value will be implemented in the future
/// like `pgp` or `smime`. The command refers to
/// [`CompilerOpts::pgp_sign_cmd].
pub(crate) fn sign() -> impl Parser<char, Prop, Error = Simple<char>> {
    just(SIGN)
        .then_ignore(just('=').padded())
        .then(maybe_quoted_val("command"))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}
