mod parts;
mod props;
mod tpl;
mod vals;

pub(crate) mod prelude {
    pub(crate) use chumsky::prelude::*;
}

pub(crate) use parts::*;
pub(crate) use props::*;
pub(crate) use tpl::*;
pub(crate) use vals::*;

pub(crate) const BACKSLASH: char = '\\';
pub(crate) const COLON: char = ':';
pub(crate) const DOUBLE_QUOTE: char = '"';
pub(crate) const GREATER_THAN: char = '>';
pub(crate) const SPACE: char = ' ';
