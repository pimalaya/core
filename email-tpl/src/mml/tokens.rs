use log::warn;
use mail_builder::mime::MimePart;
use pimalaya_process::Cmd;
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

    pub(crate) fn sign(part: Vec<u8>, sign_cmd: Cmd) -> pimalaya_process::Result<MimePart<'a>> {
        let signature = sign_cmd.run_with(&part)?.stdout;

        let part = MimePart::new_multipart(
            "multipart/signed; protocol=\"application/pgp-signed\"; micalg=\"pgp-sha1\"",
            vec![
                MimePart::new_binary("application/pgp-signed", part),
                MimePart::new_binary("application/pgp-signature", signature),
            ],
        );

        Ok(part)
    }

    pub(crate) fn encrypt<P: AsRef<[u8]>>(
        part: P,
        encrypt_cmd: Cmd,
    ) -> pimalaya_process::Result<MimePart<'a>> {
        let encrypted_part = encrypt_cmd.run_with(part)?.stdout;

        let part = MimePart::new_multipart(
            "multipart/encrypted; protocol=\"application/pgp-encrypted\"",
            vec![
                MimePart::new_binary("application/pgp-encrypted", "Version: 1".as_bytes()),
                MimePart::new_binary("application/octet-stream", encrypted_part),
            ],
        );

        Ok(part)
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
