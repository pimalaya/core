use async_recursion::async_recursion;
use log::{debug, warn};
use mail_builder::MessageBuilder;
use mail_parser::{Message, MessagePart, MimeHeaders, PartType};
use nanohtml2text::html2text;
use std::{env, fs, io, path::PathBuf};
use thiserror::Error;

use crate::{Pgp, Result};

use super::parsers::{
    MULTI_PART_BEGIN, MULTI_PART_BEGIN_ESCAPED, MULTI_PART_END, MULTI_PART_END_ESCAPED,
    SINGLE_PART_BEGIN, SINGLE_PART_BEGIN_ESCAPED, SINGLE_PART_END, SINGLE_PART_END_ESCAPED,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
    #[error("cannot save attachement at {1}")]
    WriteAttachmentError(#[source] io::Error, PathBuf),
    #[error("cannot build email")]
    WriteMessageError(#[source] io::Error),
    #[error("cannot parse pgp decrypted part")]
    ParsePgpDecryptedPartError,
    #[error("cannot decrypt part using pgp: missing recipient")]
    PgpDecryptMissingRecipientError,
}

/// Filters parts to show by MIME type.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum FilterParts {
    /// Shows all parts. This filter enables MML markup since multiple
    /// parts with different MIME types can be mixed together, which
    /// can be hard to navigate through.
    #[default]
    All,
    /// Shows only parts matching the given MIME type. This filter
    /// disables MML markup since only one MIME type is shown.
    Only(String),
    /// Shows only parts matching the given list of MIME types. This
    /// filter enables MML markup since multiple parts with different
    /// MIME types can be mixed together, which can be hard to
    /// navigate through.
    Include(Vec<String>),
    /// Shows all parts except those matching the given list of MIME
    /// types. This filter enables MML markup since multiple parts
    /// with different MIME types can be mixed together, which can be
    /// hard to navigate through.
    Exclude(Vec<String>),
}

impl FilterParts {
    pub fn only(&self, ctype: impl AsRef<str>) -> bool {
        match self {
            Self::All => false,
            Self::Only(this_ctype) => this_ctype == ctype.as_ref(),
            Self::Include(_) => false,
            Self::Exclude(_) => false,
        }
    }

    pub fn contains(&self, ctype: impl ToString + AsRef<str>) -> bool {
        match self {
            Self::All => true,
            Self::Only(this_ctype) => this_ctype == ctype.as_ref(),
            Self::Include(ctypes) => ctypes.contains(&ctype.to_string()),
            Self::Exclude(ctypes) => !ctypes.contains(&ctype.to_string()),
        }
    }
}

/// The MML interpreter interprets full emails as [`crate::Tpl`]. The
/// interpreter needs to be customized first. The customization
/// follows the builder pattern. When the interpreter is customized,
/// calling any function matching `interpret_*()` consumes the
/// interpreter and generates the final [`crate::Tpl`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Interpreter {
    /// If `true` then shows multipart structure. It is useful to see
    /// how nested parts are structured. If `false` then multipart
    /// structure is flatten, which means all parts and subparts are
    /// shown at the same top level.
    show_multiparts: bool,

    /// Filters parts to show by MIME type.
    filter_parts: FilterParts,

    /// If `false` then tries to remove signatures for text plain
    /// parts starting by the standard delimiter `-- \n`.
    show_plain_texts_signature: bool,

    /// If `true` then shows attachments at the end of the body as MML
    /// part.
    show_attachments: bool,

    /// If `true` then shows inline attachments at the end of the body
    /// as MML part.
    show_inline_attachments: bool,

    /// An attachment is interpreted this way: `<#part
    /// filename=attachment.ext>`. If `true` then the file (with its
    /// content) is automatically created at the given
    /// filename. Directory can be customized via
    /// `save_attachments_dir`. This option is particularly useful
    /// when transfering an email with its attachments.
    save_attachments: bool,

    /// Saves attachments to the given directory instead of the
    /// default temporary one given by [`std::env::temp_dir()`].
    save_attachments_dir: PathBuf,

    pgp: Pgp,
    pgp_sender: Option<String>,
    pgp_recipient: Option<String>,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            show_multiparts: false,
            filter_parts: Default::default(),
            show_plain_texts_signature: true,
            show_attachments: true,
            show_inline_attachments: true,
            save_attachments: Default::default(),
            save_attachments_dir: env::temp_dir(),
            pgp: Default::default(),
            pgp_sender: Default::default(),
            pgp_recipient: Default::default(),
        }
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_multiparts(mut self, b: bool) -> Self {
        self.show_multiparts = b;
        self
    }

    pub fn filter_parts(mut self, f: FilterParts) -> Self {
        self.filter_parts = f;
        self
    }

    pub fn show_plain_texts_signature(mut self, b: bool) -> Self {
        self.show_plain_texts_signature = b;
        self
    }

    pub fn show_attachments(mut self, b: bool) -> Self {
        self.show_attachments = b;
        self
    }

    pub fn show_inline_attachments(mut self, b: bool) -> Self {
        self.show_inline_attachments = b;
        self
    }

    pub fn save_attachments(mut self, b: bool) -> Self {
        self.save_attachments = b;
        self
    }

    pub fn save_attachments_dir<D>(mut self, dir: D) -> Self
    where
        D: Into<PathBuf>,
    {
        self.save_attachments_dir = dir.into();
        self
    }

    pub fn with_pgp(mut self, pgp: Pgp) -> Self {
        self.pgp = pgp;
        self
    }

    pub fn with_pgp_sender(mut self, sender: Option<String>) -> Self {
        self.pgp_sender = sender;
        self
    }

    pub fn with_pgp_recipient(mut self, recipient: Option<String>) -> Self {
        self.pgp_recipient = recipient;
        self
    }

    fn escape_mml_markup(text: String) -> String {
        text.replace(SINGLE_PART_BEGIN, SINGLE_PART_BEGIN_ESCAPED)
            .replace(SINGLE_PART_END, SINGLE_PART_END_ESCAPED)
            .replace(MULTI_PART_BEGIN, MULTI_PART_BEGIN_ESCAPED)
            .replace(MULTI_PART_END, MULTI_PART_END_ESCAPED)
    }

    async fn decrypt_part(&self, encrypted_part: &MessagePart<'_>) -> Result<String> {
        let recipient = self
            .pgp_recipient
            .as_ref()
            .ok_or(Error::PgpDecryptMissingRecipientError)?;
        let encrypted_bytes = encrypted_part.contents().to_owned();
        let decrypted_part = self.pgp.decrypt(recipient, encrypted_bytes).await?;
        let clear_part =
            Message::parse(&decrypted_part).ok_or(Error::ParsePgpDecryptedPartError)?;
        let tpl = self.interpret_msg(&clear_part).await?;
        Ok(tpl)
    }

    async fn verify_msg(&self, msg: &Message<'_>, ids: &[usize]) -> Result<()> {
        let signed_part = msg.part(ids[0]).unwrap();
        let signed_part_bytes = msg.raw_message
            [signed_part.raw_header_offset()..signed_part.raw_end_offset()]
            .to_owned();

        let signature_part = msg.part(ids[1]).unwrap();
        let signature_bytes = signature_part.contents().to_owned();

        let recipient = self
            .pgp_recipient
            .as_ref()
            .ok_or(Error::PgpDecryptMissingRecipientError)?;
        self.pgp
            .verify(recipient, signature_bytes, signed_part_bytes)
            .await?;

        Ok(())
    }

    fn interpret_attachment(&self, ctype: &str, part: &MessagePart, data: &[u8]) -> Result<String> {
        let mut tpl = String::new();

        if self.show_attachments && self.filter_parts.contains(&ctype) {
            let fname = self
                .save_attachments_dir
                .join(part.attachment_name().unwrap_or("noname"));

            if self.save_attachments {
                fs::write(&fname, data)
                    .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
            }

            let fname = fname.to_string_lossy();
            tpl = format!("<#part type={ctype} filename=\"{fname}\">\n");
        }

        Ok(tpl)
    }

    fn interpret_inline_attachment(
        &self,
        ctype: &str,
        part: &MessagePart,
        data: &[u8],
    ) -> Result<String> {
        let mut tpl = String::new();

        if self.show_inline_attachments && self.filter_parts.contains(&ctype) {
            let ctype = get_ctype(part);
            let fname = self.save_attachments_dir.join(
                part.attachment_name()
                    .or(part.content_id())
                    .unwrap_or("noname"),
            );

            if self.save_attachments {
                fs::write(&fname, data)
                    .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
            }

            let fname = fname.to_string_lossy();
            tpl = format!("<#part type={ctype} disposition=inline filename=\"{fname}\">\n");
        }

        Ok(tpl)
    }

    fn interpret_text(&self, ctype: &str, text: &str) -> String {
        let mut tpl = String::new();

        if self.filter_parts.contains(ctype) {
            let text = text.replace("\r", "");
            let text = Self::escape_mml_markup(text);

            if self.filter_parts.only(&ctype) {
                tpl.push_str(&text);
            } else {
                tpl.push_str(&format!("<#part type={ctype}>\n"));
                tpl.push_str(&text);
                tpl.push_str("<#/part>\n");
            }
        }

        tpl
    }

    fn interpret_text_plain(&self, plain: &str) -> String {
        let mut tpl = String::new();

        if self.filter_parts.contains("text/plain") {
            let plain = plain.replace("\r", "");
            let mut plain = Self::escape_mml_markup(plain);

            if !self.show_plain_texts_signature {
                plain = plain
                    .rsplit_once("-- \n")
                    .map(|(body, _signature)| body.to_owned())
                    .unwrap_or(plain);
            }

            tpl.push_str(&plain);
        }

        tpl
    }

    fn interpret_text_html(&self, html: &str) -> String {
        let mut tpl = String::new();

        if self.filter_parts.contains("text/html") {
            if self.filter_parts.only("text/html") {
                let html = html.replace("\r", "");
                let html = Self::escape_mml_markup(html);
                tpl.push_str(&html);
            } else {
                let html = html2text(html);
                let html = Self::escape_mml_markup(html);
                tpl.push_str("<#part type=text/html>\n");
                tpl.push_str(&html);
                tpl.push_str("<#/part>\n");
            }
        }

        tpl
    }

    #[async_recursion]
    async fn interpret_part(&self, msg: &Message<'_>, part: &MessagePart<'_>) -> Result<String> {
        let mut tpl = String::new();
        let ctype = get_ctype(part);

        match &part.body {
            PartType::Text(plain) if ctype == "text/plain" => {
                tpl.push_str(&self.interpret_text_plain(plain));
            }
            PartType::Text(text) => {
                tpl.push_str(&self.interpret_text(&ctype, text));
            }
            PartType::Html(html) => {
                tpl.push_str(&self.interpret_text_html(html));
            }
            PartType::Binary(data) => {
                tpl.push_str(&self.interpret_attachment(&ctype, part, data)?);
            }
            PartType::InlineBinary(data) => {
                tpl.push_str(&self.interpret_inline_attachment(&ctype, part, data)?);
            }
            PartType::Message(msg) => {
                tpl.push_str(&self.interpret_msg(msg).await?);
            }
            PartType::Multipart(ids) if ctype == "multipart/alternative" => {
                let mut parts = ids.into_iter().filter_map(|id| msg.part(*id));

                let part = match &self.filter_parts {
                    FilterParts::All => {
                        let part = parts
                            .clone()
                            .find_map(|part| match &part.body {
                                PartType::Text(plain)
                                    if is_plain(part) && !plain.trim().is_empty() =>
                                {
                                    Some(Ok(self.interpret_text_plain(plain)))
                                }
                                _ => None,
                            })
                            .or_else(|| {
                                parts.clone().find_map(|part| match &part.body {
                                    PartType::Html(html) if !html.trim().is_empty() => {
                                        Some(Ok(self.interpret_text_html(html)))
                                    }
                                    _ => None,
                                })
                            })
                            .or_else(|| {
                                parts.clone().find_map(|part| {
                                    let ctype = get_ctype(part);
                                    match &part.body {
                                        PartType::Text(text) if !text.trim().is_empty() => {
                                            Some(Ok(self.interpret_text(&ctype, text)))
                                        }
                                        _ => None,
                                    }
                                })
                            });

                        match part {
                            Some(part) => Some(part),
                            None => match parts.next() {
                                Some(part) => Some(self.interpret_part(msg, part).await),
                                None => None,
                            },
                        }
                    }
                    FilterParts::Only(ctype) => {
                        match parts.clone().find(|part| &get_ctype(part) == ctype) {
                            Some(part) => Some(self.interpret_part(msg, part).await),
                            None => None,
                        }
                    }
                    FilterParts::Include(ctypes) => {
                        match parts.clone().find(|part| ctypes.contains(&get_ctype(part))) {
                            Some(part) => Some(self.interpret_part(msg, part).await),
                            None => None,
                        }
                    }
                    FilterParts::Exclude(ctypes) => {
                        match parts
                            .clone()
                            .find(|part| !ctypes.contains(&get_ctype(part)))
                        {
                            Some(part) => Some(self.interpret_part(msg, part).await),
                            None => None,
                        }
                    }
                };

                if let Some(part) = part {
                    tpl.push_str(&part?);
                }
            }
            PartType::Multipart(ids) if ctype == "multipart/encrypted" => {
                match self.decrypt_part(msg.part(ids[1]).unwrap()).await {
                    Ok(ref clear_part) => tpl.push_str(clear_part),
                    Err(err) => {
                        warn!("cannot decrypt email part using pgp: {err}");
                        debug!("cannot decrypt email part using pgp: {err:?}");
                    }
                }
            }
            PartType::Multipart(ids) if ctype == "multipart/signed" => {
                match self.verify_msg(msg, &ids).await {
                    Ok(()) => {
                        debug!("email part successfully verified using pgp");
                    }
                    Err(err) => {
                        warn!("cannot verify email part using pgp: {err}");
                        debug!("cannot verify email part using pgp: {err:?}");
                    }
                }

                let signed_part = msg.part(ids[0]).unwrap();
                let clear_part = &self.interpret_part(msg, signed_part).await?;
                tpl.push_str(clear_part);
            }
            PartType::Multipart(_) if ctype == "application/pgp-encrypted" => {
                // TODO: check if content matches "Version: 1"
            }
            PartType::Multipart(_) if ctype == "application/pgp-signature" => {
                // nothing to do, signature already verified above
            }
            PartType::Multipart(ids) => {
                if self.show_multiparts {
                    let stype = part
                        .content_type()
                        .and_then(|p| p.subtype())
                        .unwrap_or("mixed");
                    tpl.push_str(&format!("<#multipart type={stype}>\n"));
                }

                for id in ids {
                    if let Some(part) = msg.part(*id) {
                        tpl.push_str(&self.interpret_part(msg, part).await?);
                    } else {
                        warn!("cannot find part {id}, skipping it");
                    }
                }

                if self.show_multiparts {
                    tpl.push_str("<#/multipart>\n");
                }
            }
        }

        Ok(tpl)
    }

    /// Interprets the given [`mail_parser::Message`] as a MML string.
    pub async fn interpret_msg<'a>(&self, msg: &Message<'a>) -> Result<String> {
        self.interpret_part(msg, msg.root_part()).await
    }

    /// Interprets the given bytes as a MML string.
    pub async fn interpret_bytes<'a>(&self, bytes: impl AsRef<[u8]> + 'a) -> Result<String> {
        let msg = Message::parse(bytes.as_ref()).ok_or(Error::ParseRawEmailError)?;
        self.interpret_msg(&msg).await
    }

    /// Interprets the given [`mail_builder::MessageBuilder`] as a MML
    /// string.
    pub async fn interpret_msg_builder<'a>(&self, builder: MessageBuilder<'a>) -> Result<String> {
        let bytes = builder.write_to_vec().map_err(Error::WriteMessageError)?;
        self.interpret_bytes(&bytes).await
    }
}

fn get_ctype(part: &MessagePart) -> String {
    part.content_type()
        .and_then(|ctype| {
            ctype
                .subtype()
                .map(|stype| format!("{}/{stype}", ctype.ctype()))
        })
        .unwrap_or_else(|| String::from("application/octet-stream"))
}

fn is_plain(part: &MessagePart) -> bool {
    get_ctype(part) == "text/plain"
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use mail_builder::{mime::MimePart, MessageBuilder};

    use super::{FilterParts, Interpreter};

    #[tokio::test]
    async fn nested_multiparts() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new(
                    "multipart/related",
                    vec![
                        MimePart::new("text/plain", "\nThis is a second plain text part.\n\n"),
                        MimePart::new("text/plain", "This is a third plain text part.\n\n\n"),
                    ],
                ),
            ],
        ));

        let tpl = Interpreter::new()
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "This is a plain text part.",
            "",
            "This is a second plain text part.",
            "",
            "This is a third plain text part.",
            "",
            "",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn nested_multiparts_with_markup() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n\n"),
                MimePart::new(
                    "multipart/related",
                    vec![
                        MimePart::new("text/plain", "This is a second plain text part.\n\n"),
                        MimePart::new("text/plain", "This is a third plain text part.\n\n"),
                    ],
                ),
            ],
        ));

        let tpl = Interpreter::new()
            .show_multiparts(true)
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "<#multipart type=mixed>",
            "This is a plain text part.",
            "",
            "<#multipart type=related>",
            "This is a second plain text part.",
            "",
            "This is a third plain text part.",
            "",
            "<#/multipart>",
            "<#/multipart>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn all_text() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n\n"),
                MimePart::new("text/html", "<h1>This is a &lt;HTML&gt; text part.</h1>\n"),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "This is a plain text part.",
            "",
            "<#part type=text/html>",
            "This is a <HTML> text part.\r",
            "\r",
            "<#/part>",
            "<#part type=text/json>",
            "{\"type\": \"This is a JSON text part.\"}",
            "<#/part>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn only_text_plain() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new(
                    "text/html",
                    "<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>\n",
                ),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .filter_parts(FilterParts::Only("text/plain".into()))
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!("This is a plain text part.", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn only_text_html() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new(
                    "text/html",
                    "<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>\n",
                ),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .filter_parts(FilterParts::Only("text/html".into()))
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!("<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn only_text_other() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new(
                    "text/html",
                    "<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>\n",
                ),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .filter_parts(FilterParts::Only("text/json".into()))
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!("{\"type\": \"This is a JSON text part.\"}", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn multipart_alternative_text_all_without_plain() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/alternative",
            vec![
                MimePart::new("text/html", "<h1>This is a &lt;HTML&gt; text part.</h1>\n"),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "<#part type=text/html>",
            "This is a <HTML> text part.\r",
            "\r",
            "<#/part>",
            ""
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn multipart_alternative_text_all_with_empty_plain() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/alternative",
            vec![
                MimePart::new("text/plain", "    \n\n"),
                MimePart::new("text/html", "<h1>This is a &lt;HTML&gt; text part.</h1>\n"),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "<#part type=text/html>",
            "This is a <HTML> text part.\r",
            "\r",
            "<#/part>",
            ""
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn multipart_alternative_text_all_without_plain_nor_html() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/alternative",
            vec![MimePart::new(
                "text/json",
                "{\"type\": \"This is a JSON text part.\"}\n",
            )],
        ));

        let tpl = Interpreter::new()
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "<#part type=text/json>",
            "{\"type\": \"This is a JSON text part.\"}",
            "<#/part>",
            ""
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn multipart_alternative_text_all() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/alternative",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new(
                    "text/html",
                    "<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>\n",
                ),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!("This is a plain text part.", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn multipart_alternative_text_html_only() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/alternative",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new(
                    "text/html",
                    "<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>\n",
                ),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = Interpreter::new()
            .filter_parts(FilterParts::Only("text/html".into()))
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!("<h1>This is a &lt;HTML&gt; text&nbsp;part.</h1>", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn attachment() {
        let builder = MessageBuilder::new().attachment(
            "application/octet-stream",
            "attachment.txt",
            "Hello, world!".as_bytes(),
        );

        let tpl = Interpreter::new()
            .save_attachments_dir("~/Downloads")
            .interpret_msg_builder(builder)
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "<#part type=application/octet-stream filename=\"~/Downloads/attachment.txt\">",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }
}
