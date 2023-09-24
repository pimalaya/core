mod parts;
mod props;
mod vals;

pub(crate) mod prelude {
    use crate::message::body::{
        BACKSLASH, DOUBLE_QUOTE, MULTIPART_BEGIN, MULTIPART_END, NEW_LINE, PART_BEGIN, PART_END,
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
}

pub(crate) use parts::*;
pub(crate) use props::*;
pub(crate) use vals::*;
