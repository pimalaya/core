use async_recursion::async_recursion;
use log::warn;
use mail_builder::{mime::MimePart, MessageBuilder};
use pimalaya_pgp::{SignedPublicKey, SignedSecretKey};
use std::{env, ffi::OsStr, fs, io, path::PathBuf};
use thiserror::Error;

use crate::{
    mml::parsers::{self, prelude::*},
    Result,
};

use super::tokens::{Part, DISPOSITION, ENCRYPT, FILENAME, NAME, SIGN, TYPE};

#[derive(Debug, Error)]
pub enum Error {
    // TODO: return the original chumsky::Error
    #[error("cannot parse MML template: {0}")]
    ParseMmlError(String),
    #[error("cannot compile template: recipient is missing")]
    CompileTplMissingRecipientError,
    #[error("cannot compile template")]
    WriteCompiledPartToVecError(#[source] io::Error),
    #[error("cannot find missing property filename")]
    GetFilenamePropMissingError,
    #[error("cannot expand filename {1}")]
    ExpandFilenameError(#[source] shellexpand::LookupError<env::VarError>, String),
    #[error("cannot read attachment at {1}")]
    ReadAttachmentError(#[source] io::Error, String),
    #[error("cannot encrypt multi part")]
    EncryptPartError(#[from] pimalaya_process::Error),
    #[error("cannot sign multi part")]
    SignPartError(#[source] pimalaya_process::Error),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Compiler {
    pgp_encrypt_keys: Option<Vec<SignedPublicKey>>,
    pgp_sign_key: Option<SignedSecretKey>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pgp_encrypt_keys(
        mut self,
        keys: Option<impl IntoIterator<Item = SignedPublicKey>>,
    ) -> Self {
        self.pgp_encrypt_keys = if let Some(keys) = keys {
            Some(keys.into_iter().collect())
        } else {
            None
        };
        self
    }

    pub fn with_pgp_sign_key(mut self, key: Option<SignedSecretKey>) -> Self {
        self.pgp_sign_key = key;
        self
    }

    async fn encrypt<'a>(&self, part: MimePart<'a>) -> Result<MimePart<'a>> {
        if let Some(pkeys) = &self.pgp_encrypt_keys {
            let mut buf = Vec::new();
            part.clone()
                .write_part(&mut buf)
                .map_err(Error::WriteCompiledPartToVecError)?;
            let encrypted_part = pimalaya_pgp::encrypt(buf, pkeys.clone()).await?;

            let part = MimePart::new(
                "multipart/encrypted; protocol=\"application/pgp-encrypted\"",
                vec![
                    MimePart::new("application/pgp-encrypted", "Version: 1"),
                    MimePart::new("application/octet-stream", encrypted_part),
                ],
            );

            Ok(part)
        } else {
            warn!("cannot encrypt email part: encrypt not set up");
            Ok(part)
        }
    }

    async fn sign<'a>(&self, part: MimePart<'a>) -> Result<MimePart<'a>> {
        if let Some(skey) = &self.pgp_sign_key {
            let mut buf = Vec::new();
            part.clone()
                .write_part(&mut buf)
                .map_err(Error::WriteCompiledPartToVecError)?;
            let signature = pimalaya_pgp::sign(buf, skey.clone()).await?;

            let part = MimePart::new(
                "multipart/signed; protocol=\"application/pgp-signature\"; micalg=\"pgp-sha1\"",
                vec![part, MimePart::new("application/pgp-signature", signature)],
            );

            Ok(part)
        } else {
            warn!("cannot sign email part: sign not set up");
            Ok(part)
        }
    }

    async fn compile_parts<'a>(&self, parts: Vec<Part>) -> Result<MessageBuilder<'a>> {
        let parts = Part::compact_text_plain_parts(parts);

        let mut builder = MessageBuilder::new();

        builder = match parts.len() {
            0 => builder.text_body(String::new()),
            1 => builder.body(self.compile_part(parts.into_iter().next().unwrap()).await?),
            _ => {
                let mut compiled_parts = Vec::new();

                for part in parts {
                    let part = self.compile_part(part).await?;
                    compiled_parts.push(part);
                }

                builder.body(MimePart::new("multipart/mixed", compiled_parts))
            }
        };

        Ok(builder)
    }

    #[async_recursion]
    async fn compile_part<'a>(&self, part: Part) -> Result<MimePart<'a>> {
        match part {
            Part::MultiPart((props, parts)) => {
                let no_parts: Vec<u8> = Vec::new();

                let mut multi_part = match props.get(TYPE).map(String::as_str) {
                    Some("mixed") | None => MimePart::new("multipart/mixed", no_parts),
                    Some("alternative") => MimePart::new("multipart/alternative", no_parts),
                    Some("related") => MimePart::new("multipart/related", no_parts),
                    Some(unknown) => {
                        warn!("unknown multipart type {unknown}, fall back to mixed");
                        MimePart::new("multipart/mixed", no_parts)
                    }
                };

                for part in Part::compact_text_plain_parts(parts) {
                    multi_part.add_part(self.compile_part(part).await?)
                }

                let multi_part = match props.get(SIGN).map(String::as_str) {
                    Some("command") => self.sign(multi_part).await,
                    _ => Ok(multi_part),
                }?;

                let multi_part = match props.get(ENCRYPT).map(String::as_str) {
                    Some("command") => self.encrypt(multi_part).await,
                    _ => Ok(multi_part),
                }?;

                Ok(multi_part)
            }
            Part::SinglePart((ref props, body)) => {
                let ctype = Part::get_or_guess_content_type(props, &body);
                let mut part = MimePart::new(ctype, body);

                part = match props.get(DISPOSITION).map(String::as_str) {
                    Some("inline") => part.inline(),
                    Some("attachment") => {
                        let fname = props
                            .get(NAME)
                            .map(ToOwned::to_owned)
                            .unwrap_or("noname".into());
                        part.attachment(fname)
                    }
                    _ => part,
                };

                part = match props.get(SIGN).map(String::as_str) {
                    Some("command") => self.sign(part).await,
                    _ => Ok(part),
                }?;

                part = match props.get(ENCRYPT).map(String::as_str) {
                    Some("command") => self.encrypt(part).await,
                    _ => Ok(part),
                }?;

                Ok(part)
            }
            Part::Attachment(ref props) => {
                let filepath = props
                    .get(FILENAME)
                    .ok_or(Error::GetFilenamePropMissingError)?;
                let filepath = shellexpand::full(&filepath)
                    .map_err(|err| Error::ExpandFilenameError(err, filepath.to_string()))?
                    .to_string();

                let body = fs::read(&filepath)
                    .map_err(|err| Error::ReadAttachmentError(err, filepath.clone()))?;

                let fname = props
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
                let content_type = Part::get_or_guess_content_type(props, &body);

                let mut part = MimePart::new(content_type, body);

                part = match disposition {
                    Some("inline") => part.inline(),
                    _ => part.attachment(fname),
                };

                part = match props.get(SIGN).map(String::as_str) {
                    Some("command") => self.sign(part).await,
                    _ => Ok(part),
                }?;

                part = match props.get(ENCRYPT).map(String::as_str) {
                    Some("command") => self.encrypt(part).await,
                    _ => Ok(part),
                }?;

                Ok(part)
            }
            Part::TextPlainPart(body) => Ok(MimePart::new("text/plain", body)),
        }
    }

    pub async fn compile<'a>(&self, tpl: impl AsRef<str>) -> Result<MessageBuilder<'a>> {
        let parts = parsers::parts()
            .parse(tpl.as_ref())
            .map_err(|errs| Error::ParseMmlError(errs[0].to_string()))?;
        self.compile_parts(parts).await
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use std::io::prelude::*;
    use tempfile::Builder;

    use super::Compiler;

    #[tokio::test]
    async fn plain() {
        let tpl = concat_line!("Hello, world!", "");

        let msg = Compiler::new()
            .compile(&tpl)
            .await
            .unwrap()
            .message_id("id@localhost")
            .date(0 as u64)
            .write_to_string()
            .unwrap();

        let expected_msg = concat_line!(
            "Message-ID: <id@localhost>\r",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000\r",
            "Content-Type: text/plain; charset=\"utf-8\"\r",
            "Content-Transfer-Encoding: 7bit\r",
            "\r",
            "Hello, world!\r",
            "",
        );

        assert_eq!(msg, expected_msg);
    }

    #[tokio::test]
    async fn html() {
        let tpl = concat_line!(
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
        );

        let msg = Compiler::new()
            .compile(&tpl)
            .await
            .unwrap()
            .message_id("id@localhost")
            .date(0 as u64)
            .write_to_string()
            .unwrap();

        let expected_msg = concat_line!(
            "Message-ID: <id@localhost>\r",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000\r",
            "Content-Type: text/html; charset=\"utf-8\"\r",
            "Content-Transfer-Encoding: 7bit\r",
            "\r",
            "<h1>Hello, world!</h1>",
        );

        assert_eq!(msg, expected_msg);
    }

    #[tokio::test]
    async fn attachment() {
        let mut attachment = Builder::new()
            .prefix("attachment")
            .suffix(".txt")
            .rand_bytes(0)
            .tempfile()
            .unwrap();
        write!(attachment, "Hello, world!").unwrap();
        let attachment_path = attachment.path().to_string_lossy();

        let tpl = format!("<#part filename=\"{attachment_path}\" type=\"text/plain\">");

        let msg = Compiler::new()
            .compile(&tpl)
            .await
            .unwrap()
            .message_id("id@localhost")
            .date(0 as u64)
            .write_to_string()
            .unwrap();

        let expected_msg = concat_line!(
            "Message-ID: <id@localhost>\r",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000\r",
            "Content-Type: text/plain\r",
            "Content-Disposition: attachment; filename=\"attachment.txt\"\r",
            "Content-Transfer-Encoding: 7bit\r",
            "\r",
            "Hello, world!",
        );

        assert_eq!(msg, expected_msg);
    }
}
