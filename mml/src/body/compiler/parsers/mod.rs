mod parts;
mod props;
mod vals;

pub(crate) mod prelude {
    pub(crate) use chumsky::prelude::*;
}

pub(crate) use parts::*;
pub(crate) use props::*;
pub(crate) use vals::*;
