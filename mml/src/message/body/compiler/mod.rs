//! # MML to MIME message body compilation module
//!
//! Module dedicated to MML → MIME message body compilation.

mod parsers;
mod tokens;

use std::{ffi::OsStr, fs, ops::Deref};

use async_recursion::async_recursion;
use mail_builder::{
    mime::{BodyPart, MimePart},
    MessageBuilder,
};
use shellexpand_utils::shellexpand_path;
#[allow(unused_imports)]
use tracing::{debug, warn};

#[cfg(feature = "pgp")]
use crate::pgp::Pgp;
use crate::{Error, Result};

use super::{
    ALTERNATIVE, ATTACHMENT, DISPOSITION, ENCODING, ENCODING_7BIT, ENCODING_8BIT, ENCODING_BASE64,
    ENCODING_QUOTED_PRINTABLE, FILENAME, INLINE, MIXED, MULTIPART_BEGIN, MULTIPART_BEGIN_ESCAPED,
    MULTIPART_END, MULTIPART_END_ESCAPED, NAME, PART_BEGIN, PART_BEGIN_ESCAPED, PART_END,
    PART_END_ESCAPED, RECIPIENT_FILENAME, RELATED, TYPE,
};
#[cfg(feature = "pgp")]
use super::{ENCRYPT, PGP_MIME, SIGN};

use self::{parsers::prelude::*, tokens::Part};

/// MML → MIME message body compiler.
///
/// The compiler follows the builder pattern, where the build function
/// is named `compile`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MmlBodyCompiler {
    #[cfg(feature = "pgp")]
    pgp: Option<Pgp>,
    #[cfg(feature = "pgp")]
    pgp_sender: Option<String>,
    #[cfg(feature = "pgp")]
    pgp_recipients: Vec<String>,
}

impl<'a> MmlBodyCompiler {
    /// Create a new MML message body compiler with default options.
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "pgp")]
    pub fn set_pgp(&mut self, pgp: impl Into<Pgp>) {
        self.pgp = Some(pgp.into());
    }

    #[cfg(feature = "pgp")]
    pub fn with_pgp(mut self, pgp: impl Into<Pgp>) -> Self {
        self.set_pgp(pgp);
        self
    }

    #[cfg(feature = "pgp")]
    pub fn set_some_pgp(&mut self, pgp: Option<impl Into<Pgp>>) {
        self.pgp = pgp.map(Into::into);
    }

    #[cfg(feature = "pgp")]
    pub fn with_some_pgp(mut self, pgp: Option<impl Into<Pgp>>) -> Self {
        self.set_some_pgp(pgp);
        self
    }

    #[cfg(feature = "pgp")]
    pub fn with_pgp_sender(mut self, sender: Option<String>) -> Self {
        self.pgp_sender = sender;
        self
    }

    #[cfg(feature = "pgp")]
    pub fn with_pgp_recipients(mut self, recipients: Vec<String>) -> Self {
        self.pgp_recipients = recipients;
        self
    }

    /// Encrypt the given MIME part using PGP.
    #[cfg(feature = "pgp")]
    async fn encrypt_part(&self, clear_part: &MimePart<'a>) -> Result<MimePart<'a>> {
        match &self.pgp {
            None => {
                debug!("cannot encrypt part: pgp not configured");
                Ok(clear_part.clone())
            }
            Some(pgp) => {
                let recipients = self.pgp_recipients.clone();

                let mut clear_part_bytes = Vec::new();
                clear_part
                    .clone()
                    .write_part(&mut clear_part_bytes)
                    .map_err(Error::WriteCompiledPartToVecError)?;

                let encrypted_part_bytes = pgp.encrypt(recipients, clear_part_bytes).await?;
                let encrypted_part_bytes =
                    encrypted_part_bytes
                        .into_iter()
                        .fold(Vec::new(), |mut part, b| {
                            if b == b'\n' {
                                part.push(b'\r');
                                part.push(b'\n');
                            } else {
                                part.push(b);
                            };
                            part
                        });
                let encrypted_part = MimePart::new(
                    "multipart/encrypted; protocol=\"application/pgp-encrypted\"",
                    vec![
                        MimePart::new("application/pgp-encrypted", "Version: 1"),
                        MimePart::new("application/octet-stream", encrypted_part_bytes)
                            .transfer_encoding("7bit"),
                    ],
                );

                Ok(encrypted_part)
            }
        }
    }

    /// Try to encrypt the given MIME part using PGP.
    ///
    /// If the operation fails, log a warning and return the original
    /// MIME part.
    #[cfg(feature = "pgp")]
    async fn try_encrypt_part(&self, clear_part: MimePart<'a>) -> MimePart<'a> {
        match self.encrypt_part(&clear_part).await {
            Ok(encrypted_part) => encrypted_part,
            Err(err) => {
                debug!("cannot encrypt email part using pgp: {err}");
                debug!("{err:?}");
                clear_part
            }
        }
    }

    /// Sign the given MIME part using PGP.
    #[cfg(feature = "pgp")]
    async fn sign_part(&self, clear_part: MimePart<'a>) -> Result<MimePart<'a>> {
        match &self.pgp {
            None => {
                debug!("cannot sign part: pgp not configured");
                Ok(clear_part.clone())
            }
            Some(pgp) => {
                let sender = self
                    .pgp_sender
                    .as_ref()
                    .ok_or(Error::PgpSignMissingSenderError)?;

                let mut clear_part_bytes = Vec::new();
                clear_part
                    .clone()
                    .write_part(&mut clear_part_bytes)
                    .map_err(Error::WriteCompiledPartToVecError)?;

                let signature_bytes = pgp.sign(sender, clear_part_bytes).await?;
                let signature_bytes =
                    signature_bytes.into_iter().fold(Vec::new(), |mut part, b| {
                        if b == b'\n' {
                            part.push(b'\r');
                            part.push(b'\n');
                        } else {
                            part.push(b);
                        };
                        part
                    });

                let signed_part = MimePart::new(
                    "multipart/signed; protocol=\"application/pgp-signature\"; micalg=\"pgp-sha256\"",
                    vec![
                        clear_part,
                        MimePart::new("application/pgp-signature", signature_bytes)
                            .transfer_encoding("7bit"),
                    ],
                );

                Ok(signed_part)
            }
        }
    }

    /// Try to sign the given MIME part using PGP.
    ///
    /// If the operation fails, log a warning and return the original
    /// MIME part.
    #[cfg(feature = "pgp")]
    async fn try_sign_part(&self, clear_part: MimePart<'a>) -> MimePart<'a> {
        match self.sign_part(clear_part.clone()).await {
            Ok(signed_part) => signed_part,
            Err(err) => {
                debug!("cannot sign email part using pgp: {err}");
                debug!("{err:?}");
                clear_part
            }
        }
    }

    /// Replace escaped opening and closing tags by normal opening and
    /// closing tags.
    fn unescape_mml_markup(text: impl AsRef<str>) -> String {
        text.as_ref()
            .replace(PART_BEGIN_ESCAPED, PART_BEGIN)
            .replace(PART_END_ESCAPED, PART_END)
            .replace(MULTIPART_BEGIN_ESCAPED, MULTIPART_BEGIN)
            .replace(MULTIPART_END_ESCAPED, MULTIPART_END)
    }

    /// Compile given parts parsed from a MML body to a
    /// [MessageBuilder].
    async fn compile_parts(&'a self, parts: Vec<Part<'a>>) -> Result<MessageBuilder<'a>> {
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

    /// Compile the given part parsed from MML body to a [MimePart].
    #[async_recursion]
    async fn compile_part(&'a self, part: Part<'a>) -> Result<MimePart<'a>> {
        match part {
            Part::Multi(props, parts) => {
                let no_parts = BodyPart::Multipart(Vec::new());

                let mut multi_part = match props.get(TYPE) {
                    Some(&MIXED) | None => MimePart::new("multipart/mixed", no_parts),
                    Some(&ALTERNATIVE) => MimePart::new("multipart/alternative", no_parts),
                    Some(&RELATED) => MimePart::new("multipart/related", no_parts),
                    Some(unknown) => {
                        debug!("unknown multipart type {unknown}, falling back to mixed");
                        MimePart::new("multipart/mixed", no_parts)
                    }
                };

                for part in parts {
                    multi_part.add_part(self.compile_part(part).await?)
                }

                #[cfg(feature = "pgp")]
                {
                    multi_part = match props.get(SIGN) {
                        Some(&PGP_MIME) => self.try_sign_part(multi_part).await,
                        _ => multi_part,
                    };

                    multi_part = match props.get(ENCRYPT) {
                        Some(&PGP_MIME) => self.try_encrypt_part(multi_part).await,
                        _ => multi_part,
                    };
                }

                Ok(multi_part)
            }
            Part::Single(ref props, body) => {
                let fpath = props.get(FILENAME).map(shellexpand_path);

                let mut part = match &fpath {
                    Some(fpath) => {
                        let contents = fs::read(fpath)
                            .map_err(|err| Error::ReadAttachmentError(err, fpath.clone()))?;
                        let mut ctype = Part::get_or_guess_content_type(props, &contents).into();
                        if let Some(name) = props.get(NAME) {
                            ctype = ctype.attribute("name", *name);
                        }
                        MimePart::new(ctype, contents)
                    }
                    None => {
                        let mut ctype =
                            Part::get_or_guess_content_type(props, body.as_bytes()).into();
                        if let Some(name) = props.get(NAME) {
                            ctype = ctype.attribute("name", *name);
                        }
                        MimePart::new(ctype, body)
                    }
                };

                part = match props.get(ENCODING) {
                    Some(&ENCODING_7BIT) => part.transfer_encoding(ENCODING_7BIT),
                    Some(&ENCODING_8BIT) => part.transfer_encoding(ENCODING_8BIT),
                    Some(&ENCODING_QUOTED_PRINTABLE) => {
                        part.transfer_encoding(ENCODING_QUOTED_PRINTABLE)
                    }
                    Some(&ENCODING_BASE64) => part.transfer_encoding(ENCODING_BASE64),
                    _ => part,
                };

                part = match props.get(DISPOSITION) {
                    Some(&INLINE) => part.inline(),
                    Some(&ATTACHMENT) => part.attachment(
                        props
                            .get(RECIPIENT_FILENAME)
                            .map(Deref::deref)
                            .or_else(|| match &fpath {
                                Some(fpath) => fpath.file_name().and_then(OsStr::to_str),
                                None => None,
                            })
                            .unwrap_or("noname")
                            .to_owned(),
                    ),
                    _ if fpath.is_some() => part.attachment(
                        props
                            .get(RECIPIENT_FILENAME)
                            .map(ToString::to_string)
                            .or_else(|| {
                                fpath
                                    .unwrap()
                                    .file_name()
                                    .and_then(OsStr::to_str)
                                    .map(ToString::to_string)
                            })
                            .unwrap_or_else(|| "noname".to_string()),
                    ),
                    _ => part,
                };

                #[cfg(feature = "pgp")]
                {
                    part = match props.get(SIGN) {
                        Some(&PGP_MIME) => self.try_sign_part(part).await,
                        _ => part,
                    };

                    part = match props.get(ENCRYPT) {
                        Some(&PGP_MIME) => self.try_encrypt_part(part).await,
                        _ => part,
                    };
                };

                Ok(part)
            }
            Part::PlainText(body) => {
                let body = Self::unescape_mml_markup(body);
                let part = MimePart::new("text/plain", body);
                Ok(part)
            }
        }
    }

    /// Compile the given raw MML body to MIME body.
    pub async fn compile(&'a self, mml_body: &'a str) -> Result<MessageBuilder<'a>> {
        let res = parsers::parts().parse(mml_body);
        if let Some(parts) = res.output() {
            Ok(self.compile_parts(parts.to_owned()).await?)
        } else {
            let errs = res.errors().map(|err| err.clone().into_owned()).collect();
            Err(Error::ParseMmlError(errs, mml_body.to_owned()))
        }
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use std::io::prelude::*;
    use tempfile::Builder;

    use super::MmlBodyCompiler;

    #[tokio::test]
    async fn plain() {
        let mml_body = concat_line!("Hello, world!", "");

        let msg = MmlBodyCompiler::new()
            .compile(mml_body)
            .await
            .unwrap()
            .message_id("id@localhost")
            .date(0_u64)
            .write_to_string()
            .unwrap();

        let expected_msg = concat_line!(
            "Message-ID: <id@localhost>\r",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000\r",
            "MIME-Version: 1.0\r",
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
        let mml_body = concat_line!(
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
        );

        let msg = MmlBodyCompiler::new()
            .compile(mml_body)
            .await
            .unwrap()
            .message_id("id@localhost")
            .date(0_u64)
            .write_to_string()
            .unwrap();

        let expected_msg = concat_line!(
            "Message-ID: <id@localhost>\r",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000\r",
            "MIME-Version: 1.0\r",
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

        let mml_body = format!(
            "<#part filename={attachment_path} type=text/plain name=custom recipient-filename=/tmp/custom encoding=base64>discarded body<#/part>"
        );

        let msg = MmlBodyCompiler::new()
            .compile(&mml_body)
            .await
            .unwrap()
            .message_id("id@localhost")
            .date(0_u64)
            .write_to_string()
            .unwrap();

        let expected_msg = concat_line!(
            "Message-ID: <id@localhost>\r",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000\r",
            "MIME-Version: 1.0\r",
            "Content-Type: text/plain; name=\"custom\"\r",
            "Content-Transfer-Encoding: base64\r",
            "Content-Disposition: attachment; filename=\"/tmp/custom\"\r",
            "\r",
            "Hello, world!",
        );

        assert_eq!(msg, expected_msg);
    }
}
