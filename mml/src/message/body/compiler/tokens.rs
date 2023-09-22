use log::warn;
use std::{borrow::Cow, collections::HashMap};

use super::TYPE;

pub(crate) type Key<'a> = &'a str;
pub(crate) type Val<'a> = &'a str;
pub(crate) type Body<'a> = &'a str;
pub(crate) type Prop<'a> = (Key<'a>, Val<'a>);
pub(crate) type Props<'a> = HashMap<Key<'a>, Val<'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Part<'a> {
    MultiPart(Props<'a>, Vec<Part<'a>>),
    SinglePart(Props<'a>, Body<'a>),
    Attachment(Props<'a>),
    TextPlainPart(Body<'a>),
}

impl<'a> Part<'a> {
    pub(crate) fn get_or_guess_content_type(
        props: &Props<'a>,
        body: impl AsRef<[u8]>,
    ) -> Cow<'a, str> {
        props
            .get(TYPE)
            .map(|t| Cow::Borrowed(*t))
            .unwrap_or_else(|| {
                let ctype = tree_magic_mini::from_u8(body.as_ref());
                warn!("no content type found, guessing from body: {ctype}");
                Cow::Owned(ctype.to_owned())
            })
    }
}
