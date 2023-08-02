use log::warn;
use std::collections::HashMap;
use tree_magic;

pub(crate) const ALTERNATIVE: &str = "alternative";
pub(crate) const ATTACHMENT: &str = "attachment";
pub(crate) const DISPOSITION: &str = "disposition";
pub(crate) const ENCRYPT: &str = "encrypt";
pub(crate) const FILENAME: &str = "filename";
pub(crate) const INLINE: &str = "inline";
pub(crate) const MIXED: &str = "mixed";
pub(crate) const NAME: &str = "name";
pub(crate) const PGP_MIME: &str = "pgpmime";
pub(crate) const RELATED: &str = "related";
pub(crate) const SIGN: &str = "sign";
pub(crate) const TYPE: &str = "type";

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

impl<'a> Part {
    pub(crate) fn get_or_guess_content_type<B>(props: &Props, body: B) -> String
    where
        B: AsRef<[u8]>,
    {
        props.get(TYPE).map(String::to_string).unwrap_or_else(|| {
            let ctype = tree_magic::from_u8(body.as_ref());
            warn!("no content type found, guessing from body: {ctype}");
            ctype
        })
    }
}
