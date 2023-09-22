//! # Property parsers
//!
//! This module contains all property parsers needed to parse MML
//! message bodies.

use crate::message::body::{
    compiler::tokens::Prop, ALTERNATIVE, ATTACHMENT, DESCRIPTION, DISPOSITION, FILENAME, INLINE,
    MIXED, NAME, RELATED, TYPE,
};
#[cfg(feature = "pgp")]
use crate::message::body::{ENCRYPT, PGP_MIME, SIGN};

use super::{maybe_quoted_const_val, prelude::*, quoted_val, val};

/// Represents the multipart type property parser. It parses value for
/// the `Content-Type` of the multipart. The value can be `mixed`,
/// `alternative` or `related`.
pub(crate) fn multipart_type<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(TYPE)
        .labelled(TYPE)
        .then_ignore(just('=').padded())
        .then(choice((
            maybe_quoted_const_val(MIXED),
            maybe_quoted_const_val(ALTERNATIVE),
            maybe_quoted_const_val(RELATED),
        )))
        .padded()
    // .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the part type property parser. It parses value for the
/// `Content-Type` header of the part.
pub(crate) fn part_type<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(TYPE)
        .labelled(TYPE)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().slice())))
        .padded()
}

/// Represents the name property parser.
pub(crate) fn name<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(NAME)
        .labelled(NAME)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().slice())))
        .padded()
}

/// Represents the description property parser.
pub(crate) fn description<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(DESCRIPTION)
        .labelled(DESCRIPTION)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().slice())))
        .padded()
}

/// Represents the disposition property parser. It parses value for
/// the `Content-Disposition` header of the part. The value can be
/// `inline` or `attachment`.
pub(crate) fn disposition<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(DISPOSITION)
        .labelled(DISPOSITION)
        .then_ignore(just('=').padded())
        .then(choice((
            maybe_quoted_const_val(INLINE),
            maybe_quoted_const_val(ATTACHMENT),
        )))
        .padded()
}

/// Represents the filename property parser.
pub(crate) fn filename<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(FILENAME)
        .labelled(FILENAME)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().slice())))
        .padded()
}

/// Represents the encrypt property parser. The value can only be
/// `command` for now, other value will be implemented in the future
/// like `pgp` or `smime`. The command refers to
/// [`CompilerOpts::pgp_encrypt_cmd`].
#[cfg(feature = "pgp")]
pub(crate) fn encrypt<'a>() -> impl Parser<'a, &'a str, Prop, ParserError<'a>> + Clone {
    just(ENCRYPT)
        .labelled(ENCRYPT)
        .then_ignore(just('=').padded())
        .then(maybe_quoted_const_val(PGP_MIME))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}

/// Represents the sign property parser. The value can only be
/// `command` for now, other value will be implemented in the future
/// like `pgp` or `smime`. The command refers to
/// [`CompilerOpts::pgp_sign_cmd].
#[cfg(feature = "pgp")]
pub(crate) fn sign<'a>() -> impl Parser<'a, &'a str, Prop, ParserError<'a>> + Clone {
    just(SIGN)
        .labelled(SIGN)
        .then_ignore(just('=').padded())
        .then(maybe_quoted_const_val(PGP_MIME))
        .padded()
        .map(|(key, val)| (key.to_string(), val.to_string()))
}
