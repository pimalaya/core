use std::collections::HashMap;

use mail_builder::headers::content_type::ContentType;
use tracing::debug;

use super::TYPE;

pub(crate) type Key<'a> = &'a str;
pub(crate) type Val<'a> = &'a str;
pub(crate) type Body<'a> = &'a str;
pub(crate) type Prop<'a> = (Key<'a>, Val<'a>);
pub(crate) type Props<'a> = HashMap<Key<'a>, Val<'a>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Part<'a> {
    Multi(Props<'a>, Vec<Part<'a>>),
    Single(Props<'a>, Body<'a>),
    PlainText(Body<'a>),
}

impl<'a> Part<'a> {
    pub(crate) fn get_or_guess_content_type(
        props: &Props,
        body: &[u8],
    ) -> impl Into<ContentType<'a>> {
        match props.get(TYPE) {
            Some(ctype) => ctype.to_string(),
            None => {
                let ctype = tree_magic_mini::from_u8(body);
                debug!("no content type found, guessing from body: {ctype}");
                ctype.to_owned()
            }
        }
    }
}
