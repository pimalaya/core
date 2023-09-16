mod parts;
mod props;
mod vals;

pub(crate) mod prelude {
    pub(crate) use chumsky::prelude::*;
    pub type ParserError<'a> = extra::Err<Rich<'a, char>>;
}

pub(crate) use parts::*;
pub(crate) use props::*;
pub(crate) use vals::*;
