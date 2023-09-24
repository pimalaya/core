mod parts;
mod props;
mod vals;

pub(crate) mod prelude {
    #[cfg(feature = "pgp")]
    use crate::message::body::PGP_MIME;
    use crate::message::body::{
        ATTACHMENT, BACKSLASH, DOUBLE_QUOTE, ENCODING_7BIT, ENCODING_8BIT, ENCODING_BASE64,
        ENCODING_QUOTED_PRINTABLE, INLINE, MULTIPART_BEGIN, MULTIPART_END, NEW_LINE, PART_BEGIN,
        PART_END,
    };

    pub(crate) use chumsky::prelude::*;

    use super::maybe_quoted_const_val;

    pub type ParserError<'a> = extra::Err<Rich<'a, char>>;

    pub(crate) fn backslash<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
        just(BACKSLASH).labelled("backslash")
    }

    pub(crate) fn dquote<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
        just(DOUBLE_QUOTE).labelled("double quote")
    }

    pub(crate) fn new_line<'a>() -> impl Parser<'a, &'a str, char, ParserError<'a>> + Clone {
        just(NEW_LINE).labelled("new line")
    }

    pub(crate) fn part_begin<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        just(PART_BEGIN).labelled("part opening tag '<#part>'")
    }

    pub(crate) fn part_end<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        just(PART_END).labelled("part closing tag '<#/part>'")
    }

    pub(crate) fn multipart_begin<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        just(MULTIPART_BEGIN).labelled("multipart opening tag '<#multipart>'")
    }

    pub(crate) fn multipart_end<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        just(MULTIPART_END).labelled("multipart closing tag '<#/multipart>'")
    }

    pub(crate) fn inline<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        maybe_quoted_const_val(INLINE).labelled(INLINE)
    }

    pub(crate) fn attachment<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        maybe_quoted_const_val(ATTACHMENT).labelled(ATTACHMENT)
    }

    pub(crate) fn encoding_7bit<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        maybe_quoted_const_val(ENCODING_7BIT).labelled(ENCODING_7BIT)
    }

    pub(crate) fn encoding_8bit<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        maybe_quoted_const_val(ENCODING_8BIT).labelled(ENCODING_8BIT)
    }

    pub(crate) fn encoding_quoted_printable<'a>(
    ) -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        maybe_quoted_const_val(ENCODING_QUOTED_PRINTABLE).labelled(ENCODING_QUOTED_PRINTABLE)
    }

    pub(crate) fn encoding_base64<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        maybe_quoted_const_val(ENCODING_BASE64).labelled(ENCODING_BASE64)
    }

    #[cfg(feature = "pgp")]
    pub(crate) fn pgp_mime<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        maybe_quoted_const_val(PGP_MIME).labelled(PGP_MIME)
    }
}

pub(crate) use parts::*;
pub(crate) use props::*;
pub(crate) use vals::*;
