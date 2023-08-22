use log::warn;
use std::collections::HashMap;
use tree_magic;

use super::TYPE;

pub(crate) type Key = String;
pub(crate) type Val = String;
pub(crate) type Prop = (Key, Val);
pub(crate) type Props = HashMap<Key, Val>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Part {
    MultiPart((Props, Vec<Part>)),
    SinglePart((Props, String)),
    Attachment(Props),
    TextPlainPart(String),
}

impl Part {
    pub(crate) fn get_or_guess_content_type(props: &Props, body: impl AsRef<[u8]>) -> String {
        props.get(TYPE).map(String::to_string).unwrap_or_else(|| {
            let ctype = tree_magic::from_u8(body.as_ref());
            warn!("no content type found, guessing from body: {ctype}");
            ctype
        })
    }
}
