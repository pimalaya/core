mod parts;
mod props;
mod vals;

pub(crate) mod prelude {
    use crate::message::body::{
        BACKSLASH, DOUBLE_QUOTE, MULTI_PART_BEGIN, MULTI_PART_END, NEW_LINE, SINGLE_PART_BEGIN,
        SINGLE_PART_END,
    };

    pub(crate) use chumsky::prelude::*;

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

    pub(crate) fn single_part_begin<'a>(
    ) -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        just(SINGLE_PART_BEGIN).labelled("single part opening tag <#part>")
    }

    pub(crate) fn single_part_end<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        just(SINGLE_PART_END).labelled("single part closing tag <#/part>")
    }

    pub(crate) fn multi_part_begin<'a>(
    ) -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone {
        just(MULTI_PART_BEGIN).labelled("multipart opening tag <#multipart>")
    }

    pub(crate) fn multi_part_end<'a>() -> impl Parser<'a, &'a str, &'a str, ParserError<'a>> + Clone
    {
        just(MULTI_PART_END).labelled("multipart closing tag <#/multipart>")
    }
}

pub(crate) use parts::*;
pub(crate) use props::*;
pub(crate) use vals::*;
