//! # Property parsers
//!
//! This module contains all property parsers needed to parse MML
//! message bodies. They mostly come from the [Emacs MML definition].
//!
//! [Emacs MML definition]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/MML-Definition.html

use crate::message::body::{
    compiler::tokens::Prop, ALTERNATIVE, CHARSET, CREATION_DATE, DATA_ENCODING, DESCRIPTION,
    DISPOSITION, ENCODING, FILENAME, MIXED, MODIFICATION_DATE, NAME, READ_DATE, RECIPIENT_FILENAME,
    RELATED, SIZE, TYPE,
};
#[cfg(feature = "pgp")]
use crate::message::body::{ENCRYPT, RECIPIENTS, SENDER, SIGN};

use super::{maybe_quoted_const_val, prelude::*, quoted_val, val};

/// The multipart type property.
///
/// > The MIME type of the part (Content-Type).
pub(crate) fn multipart_type<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(TYPE)
        .labelled(TYPE)
        .then_ignore(just('=').padded())
        .then(choice((
            maybe_quoted_const_val(MIXED).labelled(MIXED),
            maybe_quoted_const_val(ALTERNATIVE).labelled(ALTERNATIVE),
            maybe_quoted_const_val(RELATED).labelled(RELATED),
        )))
        .padded()
}

/// The part type property.
///
/// > The MIME type of the part (Content-Type).
pub(crate) fn part_type<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(TYPE)
        .labelled(TYPE)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The filename property parser.
///
/// > Use the contents of the file in the body of the part
/// (Content-Disposition).
pub(crate) fn filename<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(FILENAME)
        .labelled(FILENAME)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The recipient filename property parser.
///
/// > Use this as the file name in the generated MIME message for the
/// recipient. That is, even if the file is called foo.txt locally,
/// use this name instead in the Content-Disposition in the sent
/// message.
pub(crate) fn recipient_filename<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone
{
    just(RECIPIENT_FILENAME)
        .labelled(RECIPIENT_FILENAME)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The charset property parser.
///
/// > The contents of the body of the part are to be encoded in the
/// character set specified (Content-Type).
pub(crate) fn charset<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(CHARSET)
        .labelled(CHARSET)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The name property parser.
///
/// > Might be used to suggest a file name if the part is to be saved
/// to a file (Content-Type).
pub(crate) fn name<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(NAME)
        .labelled(NAME)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The disposition property parser.
///
/// > Valid values are ‘inline’ and ‘attachment’
/// (Content-Disposition).
pub(crate) fn disposition<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(DISPOSITION)
        .labelled(DISPOSITION)
        .then_ignore(just('=').padded())
        .then(choice((inline(), attachment())))
        .padded()
}

/// The encoding property parser.
///
/// > Valid values are ‘7bit’, ‘8bit’, ‘quoted-printable’ and
/// ‘base64’. See Charset Translation. This parameter says what
/// Content-Transfer-Encoding to use when sending the part, and is
/// normally computed automatically.
pub(crate) fn encoding<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(ENCODING)
        .labelled(ENCODING)
        .then_ignore(just('=').padded())
        .then(choice((
            encoding_7bit(),
            encoding_8bit(),
            encoding_quoted_printable(),
            encoding_base64(),
        )))
        .padded()
}

/// The data encoding property parser.
///
/// > This parameter says what encoding has been used on the data, and
/// the data will be decoded before use. Valid values are
/// ‘quoted-printable’ and ‘base64’. This is useful when you have a
/// part with binary data (for instance an image) inserted directly
/// into the Message buffer inside the ‘"<#part>...<#/part>"’ tags.
pub(crate) fn data_encoding<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(DATA_ENCODING)
        .labelled(DATA_ENCODING)
        .then_ignore(just('=').padded())
        .then(choice((encoding_quoted_printable(), encoding_base64())))
        .padded()
}

/// The description property parser.
///
/// > A description of the part (Content-Description).
pub(crate) fn description<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(DESCRIPTION)
        .labelled(DESCRIPTION)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The creation date property parser.
///
/// > Date when the part was created (Content-Disposition). This uses
/// the format of RFC 822 or its successors.
pub(crate) fn creation_date<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(CREATION_DATE)
        .labelled(CREATION_DATE)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The modification date property parser.
///
/// > RFC 822 (or later) date when the part was modified
/// (Content-Disposition).
pub(crate) fn modification_date<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone
{
    just(MODIFICATION_DATE)
        .labelled(MODIFICATION_DATE)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The read date property parser.
///
/// > RFC 822 (or later) date when the part was read
/// (Content-Disposition).
pub(crate) fn read_date<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(READ_DATE)
        .labelled(READ_DATE)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The recipients property parser.
///
/// > Who to encrypt/sign the part to. This field is used to override
/// any auto-detection based on the To/Cc headers.
#[cfg(feature = "pgp")]
pub(crate) fn recipients<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(RECIPIENTS)
        .labelled(RECIPIENTS)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The sender property parser.
///
/// > Identity used to sign the part. This field is used to override
/// the default key used.
#[cfg(feature = "pgp")]
pub(crate) fn sender<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(SENDER)
        .labelled(SENDER)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The size property parser.
///
/// > The size (in octets) of the part (Content-Disposition).
pub(crate) fn size<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(SIZE)
        .labelled(SIZE)
        .then_ignore(just('=').padded())
        .then(choice((quoted_val(), val().to_slice())))
        .padded()
}

/// The sign property parser.
///
/// What technology to sign this MML part with (smime, pgp or
/// pgpmime).
#[cfg(feature = "pgp")]
pub(crate) fn sign<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(SIGN)
        .labelled(SIGN)
        .then_ignore(just('=').padded())
        .then(pgp_mime())
        .padded()
}

/// The encrypt property parser.
///
/// > What technology to encrypt this MML part with (smime, pgp or
/// pgpmime)
#[cfg(feature = "pgp")]
pub(crate) fn encrypt<'a>() -> impl Parser<'a, &'a str, Prop<'a>, ParserError<'a>> + Clone {
    just(ENCRYPT)
        .labelled(ENCRYPT)
        .then_ignore(just('=').padded())
        .then(pgp_mime())
        .padded()
}
