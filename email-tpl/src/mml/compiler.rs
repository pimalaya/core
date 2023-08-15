use async_recursion::async_recursion;
use log::{debug, warn};
use mail_builder::{
    mime::{BodyPart, MimePart},
    MessageBuilder,
};
use std::{env, ffi::OsStr, fs, io, path::PathBuf};
use thiserror::Error;

use crate::{
    mml::parsers::{self, prelude::*},
    Pgp, Result,
};

use super::{
    parsers::{
        MULTI_PART_BEGIN, MULTI_PART_BEGIN_ESCAPED, MULTI_PART_END, MULTI_PART_END_ESCAPED,
        SINGLE_PART_BEGIN, SINGLE_PART_BEGIN_ESCAPED, SINGLE_PART_END, SINGLE_PART_END_ESCAPED,
    },
    tokens::{
        Part, ALTERNATIVE, ATTACHMENT, DISPOSITION, ENCRYPT, FILENAME, INLINE, MIXED, NAME,
        PGP_MIME, RELATED, SIGN, TYPE,
    },
};

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

    #[error("cannot sign part using pgp: missing sender")]
    PgpSignMissingSenderError,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Compiler {
    pgp: Pgp,
    pgp_sender: Option<String>,
    pgp_recipients: Vec<String>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pgp(mut self, pgp: Pgp) -> Self {
        self.pgp = pgp;
        self
    }

    pub fn with_pgp_sender(mut self, sender: Option<String>) -> Self {
        self.pgp_sender = sender;
        self
    }

    pub fn with_pgp_recipients(mut self, recipients: Vec<String>) -> Self {
        self.pgp_recipients = recipients;
        self
    }

    fn unescape_mml_markup(text: String) -> String {
        text.replace(SINGLE_PART_BEGIN_ESCAPED, SINGLE_PART_BEGIN)
            .replace(SINGLE_PART_END_ESCAPED, SINGLE_PART_END)
            .replace(MULTI_PART_BEGIN_ESCAPED, MULTI_PART_BEGIN)
            .replace(MULTI_PART_END_ESCAPED, MULTI_PART_END)
    }

    async fn encrypt_part<'a>(&self, clear_part: &MimePart<'a>) -> Result<MimePart<'a>> {
        let recipients = self.pgp_recipients.clone();

        let mut clear_part_bytes = Vec::new();
        clear_part
            .clone()
            .write_part(&mut clear_part_bytes)
            .map_err(Error::WriteCompiledPartToVecError)?;

        let encrypted_part_bytes = self.pgp.encrypt(recipients, clear_part_bytes).await?;
        let encrypted_part = MimePart::new(
            "multipart/encrypted; protocol=\"application/pgp-encrypted\"",
            vec![
                MimePart::new("application/pgp-encrypted", "Version: 1"),
                MimePart::new("application/octet-stream", encrypted_part_bytes),
            ],
        );

        Ok(encrypted_part)
    }

    async fn try_encrypt_part<'a>(&self, clear_part: MimePart<'a>) -> MimePart<'a> {
        match self.encrypt_part(&clear_part).await {
            Ok(encrypted_part) => encrypted_part,
            Err(err) => {
                warn!("cannot encrypt email part using pgp: {err}");
                debug!("cannot encrypt email part using pgp: {err:?}");
                clear_part
            }
        }
    }

    async fn sign_part<'a>(&self, clear_part: MimePart<'a>) -> Result<MimePart<'a>> {
        let sender = self
            .pgp_sender
            .as_ref()
            .ok_or(Error::PgpSignMissingSenderError)?;

        let mut clear_part_bytes = Vec::new();
        clear_part
            .clone()
            .write_part(&mut clear_part_bytes)
            .map_err(Error::WriteCompiledPartToVecError)?;

        let signature_bytes = self.pgp.sign(sender, clear_part_bytes).await?;

        let signed_part = MimePart::new(
            "multipart/signed; protocol=\"application/pgp-signature\"; micalg=\"pgp-sha1\"",
            vec![
                clear_part,
                MimePart::new("application/pgp-signature", signature_bytes),
            ],
        );

        Ok(signed_part)
    }

    async fn try_sign_part<'a>(&self, clear_part: MimePart<'a>) -> MimePart<'a> {
        match self.sign_part(clear_part.clone()).await {
            Ok(signed_part) => signed_part,
            Err(err) => {
                warn!("cannot sign email part using pgp: {err}");
                debug!("cannot sign email part using pgp: {err:?}");
                clear_part
            }
        }
    }

    async fn compile_parts<'a>(&self, parts: Vec<Part>) -> Result<MessageBuilder<'a>> {
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
                let no_parts = BodyPart::Multipart(Vec::new());

                let mut multi_part = match props.get(TYPE).map(String::as_str) {
                    Some(MIXED) | None => MimePart::new("multipart/mixed", no_parts),
                    Some(ALTERNATIVE) => MimePart::new("multipart/alternative", no_parts),
                    Some(RELATED) => MimePart::new("multipart/related", no_parts),
                    Some(unknown) => {
                        warn!("unknown multipart type {unknown}, falling back to mixed");
                        MimePart::new("multipart/mixed", no_parts)
                    }
                };

                for part in parts {
                    multi_part.add_part(self.compile_part(part).await?)
                }

                let multi_part = match props.get(SIGN).map(String::as_str) {
                    Some(PGP_MIME) => self.try_sign_part(multi_part).await,
                    _ => multi_part,
                };

                let multi_part = match props.get(ENCRYPT).map(String::as_str) {
                    Some(PGP_MIME) => self.try_encrypt_part(multi_part).await,
                    _ => multi_part,
                };

                Ok(multi_part)
            }
            Part::SinglePart((ref props, body)) => {
                let ctype = Part::get_or_guess_content_type(props, &body);
                let mut part = MimePart::new(ctype, body);

                part = match props.get(DISPOSITION).map(String::as_str) {
                    Some(INLINE) => part.inline(),
                    Some(ATTACHMENT) => {
                        let fname = props
                            .get(NAME)
                            .map(ToOwned::to_owned)
                            .unwrap_or("noname".into());
                        part.attachment(fname)
                    }
                    _ => part,
                };

                part = match props.get(SIGN).map(String::as_str) {
                    Some(PGP_MIME) => self.try_sign_part(part).await,
                    _ => part,
                };

                part = match props.get(ENCRYPT).map(String::as_str) {
                    Some(PGP_MIME) => self.try_encrypt_part(part).await,
                    _ => part,
                };

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
                    Some(INLINE) => part.inline(),
                    _ => part.attachment(fname),
                };

                part = match props.get(SIGN).map(String::as_str) {
                    Some(PGP_MIME) => self.try_sign_part(part).await,
                    _ => part,
                };

                part = match props.get(ENCRYPT).map(String::as_str) {
                    Some(PGP_MIME) => self.try_encrypt_part(part).await,
                    _ => part,
                };

                Ok(part)
            }
            Part::TextPlainPart(body) => {
                let body = Self::unescape_mml_markup(body);
                let part = MimePart::new("text/plain", body);
                Ok(part)
            }
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
            "<h1>Hello, world!</h1>\r",
            "",
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
