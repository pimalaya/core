use log::warn;
use std::collections::HashMap;
use tree_magic;

pub(crate) const DISPOSITION: &str = "disposition";
pub(crate) const ENCRYPT: &str = "encrypt";
pub(crate) const FILENAME: &str = "filename";
pub(crate) const NAME: &str = "name";
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
    pub(crate) fn get_or_guess_content_type<B: AsRef<[u8]>>(props: &Props, body: B) -> String {
        props.get(TYPE).map(String::to_string).unwrap_or_else(|| {
            let ctype = tree_magic::from_u8(body.as_ref());
            warn!("no content type found, guessing from body: {ctype}");
            ctype
        })
    }

    pub(crate) fn compact_text_plain_parts<T: AsRef<[Part]>>(parts: T) -> Vec<Part> {
        let mut compacted_plain_texts = String::default();
        let mut compacted_parts = vec![];

        for part in parts.as_ref() {
            if let Part::TextPlainPart(plain) = part {
                if !compacted_plain_texts.is_empty() {
                    compacted_plain_texts.push_str("\n\n");
                }
                compacted_plain_texts.push_str(plain);
            } else {
                compacted_parts.push(part.clone())
            }
        }

        if !compacted_plain_texts.is_empty() {
            compacted_parts.insert(0, Part::TextPlainPart(compacted_plain_texts));
        }

        compacted_parts
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::mml::tokens::Part;

    #[test]
    fn compact_text_plain_parts() {
        assert_eq!(vec![] as Vec<Part>, Part::compact_text_plain_parts(vec![]));

        assert_eq!(
            vec![Part::TextPlainPart("This is a plain text part.".into())],
            Part::compact_text_plain_parts(vec![Part::TextPlainPart(
                "This is a plain text part.".into()
            )])
        );

        assert_eq!(
            vec![Part::TextPlainPart(
                "This is a plain text part.\n\nThis is a new plain text part.".into()
            )],
            Part::compact_text_plain_parts(vec![
                Part::TextPlainPart("This is a plain text part.".into()),
                Part::TextPlainPart("This is a new plain text part.".into())
            ])
        );

        assert_eq!(
            vec![
                Part::TextPlainPart(
                    "This is a plain text part.\n\nThis is a new plain text part.".into()
                ),
                Part::SinglePart((
                    HashMap::default(),
                    "<h1>This is a HTML text part.</h1>".into()
                ))
            ],
            Part::compact_text_plain_parts(vec![
                Part::TextPlainPart("This is a plain text part.".into()),
                Part::SinglePart((
                    HashMap::default(),
                    "<h1>This is a HTML text part.</h1>".into()
                )),
                Part::TextPlainPart("This is a new plain text part.".into())
            ])
        );
    }
}
