use lettre::message::{
    header::{ContentType, ContentTypeErr},
    Attachment, MultiPart, SinglePart,
};
use log::{debug, warn};
use pimalaya_process::Cmd;
use shellexpand::{self, LookupError};
use std::{collections::HashMap, env::VarError, ffi::OsStr, fs, io, path::PathBuf};
use thiserror::Error;
use tree_magic;

use crate::{evaluator::CompilerOpts, Result};

use super::tpl::{Key, Val};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot expand filename {1}")]
    ExpandFilenameError(#[source] LookupError<VarError>, String),
    #[error("cannot find missing property filename")]
    GetFilenamePropMissingError,
    #[error("cannot parse content type {1}")]
    ParseContentTypeError(#[source] ContentTypeErr, String),
    #[error("cannot read attachment at {1}")]
    ReadAttachmentError(#[source] io::Error, String),
    #[error("cannot encrypt multi part")]
    EncryptPartError(#[from] pimalaya_process::Error),
    #[error("cannot sign multi part")]
    SignPartError(#[source] pimalaya_process::Error),
}

pub(crate) const DISPOSITION: &str = "disposition";
pub(crate) const ENCRYPT: &str = "encrypt";
pub(crate) const FILENAME: &str = "filename";
pub(crate) const NAME: &str = "name";
pub(crate) const SIGN: &str = "sign";
pub(crate) const TYPE: &str = "type";

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
    fn get_or_guess_content_type<B: AsRef<[u8]>>(props: &Props, body: B) -> Result<ContentType> {
        let content_type = props.get(TYPE).map(String::to_string).unwrap_or_else(|| {
            let content_type = tree_magic::from_u8(body.as_ref());
            debug!("no content type found, guessing {} from body", content_type);
            content_type
        });
        let content_type = ContentType::parse(&content_type)
            .map_err(|err| Error::ParseContentTypeError(err, content_type.clone()))?;

        Ok(content_type)
    }

    fn sign(formatted_part: Vec<u8>, opts: &CompilerOpts) -> Result<MultiPart> {
        let signature = Cmd::from(opts.pgp_sign_cmd.clone())
            .run_with(formatted_part.clone())
            .map_err(Error::SignPartError)?
            .stdout;

        let part = MultiPart::signed("application/pgp-signed".into(), "pgp-sha1".into())
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::parse("application/pgp-signed").unwrap())
                    .body(formatted_part),
            )
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::parse("application/pgp-signature").unwrap())
                    .body(signature),
            );

        Ok(part)
    }

    fn encrypt<P: AsRef<[u8]>>(formatted_part: P, opts: &CompilerOpts) -> Result<MultiPart> {
        let encrypted_part = Cmd::from(opts.pgp_encrypt_cmd()?)
            .run_with(formatted_part)
            .map_err(Error::EncryptPartError)?
            .stdout;

        let part = MultiPart::encrypted(String::from("application/pgp-encrypted"))
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::parse("application/pgp-encrypted").unwrap())
                    .body(String::from("Version: 1")),
            )
            .singlepart(
                SinglePart::builder()
                    .header(ContentType::parse("application/octet-stream").unwrap())
                    .body(encrypted_part),
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

    pub(crate) fn compile(&self, opts: &CompilerOpts) -> Result<PartKind> {
        match self {
            Self::MultiPart((props, parts)) => {
                let mut multi_part = match props.get(TYPE).map(String::as_str) {
                    Some("mixed") | None => MultiPart::mixed().build(),
                    Some("alternative") => MultiPart::alternative().build(),
                    Some("related") => MultiPart::related().build(),
                    Some(unknown) => {
                        warn!("unknown multipart type {}, falling back to mixed", unknown);
                        MultiPart::mixed().build()
                    }
                };

                for part in Self::compact_text_plain_parts(parts) {
                    multi_part = match part.compile(opts)? {
                        PartKind::Single(part) => multi_part.singlepart(part),
                        PartKind::Multi(part) => multi_part.multipart(part),
                    };
                }

                let multi_part = match props.get(SIGN).map(String::as_str) {
                    Some("command") => Self::sign(multi_part.formatted(), opts),
                    _ => Ok(multi_part),
                }?;

                let multi_part = match props.get(ENCRYPT).map(String::as_str) {
                    Some("command") => Self::encrypt(multi_part.formatted(), opts),
                    _ => Ok(multi_part),
                }?;

                Ok(PartKind::Multi(multi_part))
            }
            Self::SinglePart((props, body)) => {
                let content_type = Self::get_or_guess_content_type(props, body)?;
                let part = match props.get(DISPOSITION).map(String::as_str) {
                    Some("inline") => {
                        let name = props
                            .get(NAME)
                            .map(ToOwned::to_owned)
                            .unwrap_or("noname".into());
                        Attachment::new_inline(name).body(body.clone(), content_type)
                    }
                    Some("attachment") => {
                        let name = props
                            .get(NAME)
                            .map(ToOwned::to_owned)
                            .unwrap_or("noname".into());
                        Attachment::new(name).body(body.clone(), content_type)
                    }
                    None | _ => SinglePart::builder()
                        .content_type(content_type)
                        .body(body.clone()),
                };

                let part = match props.get(SIGN).map(String::as_str) {
                    Some("command") => PartKind::Multi(Self::sign(part.formatted(), opts)?),
                    _ => PartKind::Single(part),
                };

                match props.get(ENCRYPT).map(String::as_str) {
                    Some("command") => Ok(PartKind::Multi(Self::encrypt(part.formatted(), opts)?)),
                    _ => Ok(part),
                }
            }
            Self::Attachment(props) => {
                let filepath = props
                    .get(FILENAME)
                    .ok_or(Error::GetFilenamePropMissingError)?;
                let filepath = shellexpand::full(&filepath)
                    .map_err(|err| Error::ExpandFilenameError(err, filepath.to_string()))?
                    .to_string();

                let body = fs::read(&filepath)
                    .map_err(|err| Error::ReadAttachmentError(err, filepath.clone()))?;

                let name = props
                    .get(NAME)
                    .map(ToOwned::to_owned)
                    .or_else(|| {
                        PathBuf::from(filepath)
                            .file_name()
                            .and_then(OsStr::to_str)
                            .map(ToOwned::to_owned)
                    })
                    .unwrap_or("noname".into());

                let disposition = props.get(DISPOSITION).map(String::as_str);
                let attachment = if let Some("inline") = disposition {
                    Attachment::new_inline(name)
                } else {
                    Attachment::new(name)
                };

                let content_type = Self::get_or_guess_content_type(props, &body)?;
                let part = attachment.body(body, content_type);

                // sign part
                let part = match props.get(SIGN).map(String::as_str) {
                    Some("command") => PartKind::Multi(Self::sign(part.formatted(), opts)?),
                    _ => PartKind::Single(part),
                };

                // encrypt part
                match props.get(ENCRYPT).map(String::as_str) {
                    Some("command") => Ok(PartKind::Multi(Self::encrypt(part.formatted(), opts)?)),
                    _ => Ok(part),
                }
            }
            Self::TextPlainPart(body) => Ok(PartKind::Single(SinglePart::plain(body.clone()))),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum PartKind {
    Single(SinglePart),
    Multi(MultiPart),
}

impl PartKind {
    fn formatted(&self) -> Vec<u8> {
        match self {
            Self::Single(part) => part.formatted(),
            Self::Multi(part) => part.formatted(),
        }
    }
}

#[cfg(test)]
mod part {
    use io::prelude::*;
    use tempfile::NamedTempFile;

    use crate::parser::{self, prelude::*};

    use super::*;

    #[test]
    fn attachment() {
        let mut attachment = NamedTempFile::new().unwrap();
        write!(attachment, "body").unwrap();

        let part = parser::attachment()
            .parse(format!(
                "<#part name=custom filename={} type=application/octet-stream>",
                attachment.path().to_string_lossy()
            ))
            .unwrap()
            .compile(&CompilerOpts::default())
            .unwrap();

        match part {
            PartKind::Single(part) => {
                assert_eq!(
                    String::from_utf8_lossy(&part.formatted()),
		    "Content-Disposition: attachment; filename=\"custom\"\r\nContent-Type: application/octet-stream\r\nContent-Transfer-Encoding: 7bit\r\n\r\nbody\r\n",
		);
            }
            PartKind::Multi(_) => {
                panic!("attachment should not compile to a multi part")
            }
        }
    }

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
