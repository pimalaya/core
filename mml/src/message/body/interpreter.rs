//! # MIME to MML message body interpretation module
//!
//! Module dedicated to MIME → MML message body interpretation.

use std::{env, fs, path::PathBuf};

use async_recursion::async_recursion;
use mail_builder::MessageBuilder;
use mail_parser::{Message, MessageParser, MessagePart, MimeHeaders, PartType};
use nanohtml2text::html2text;
#[allow(unused_imports)]
use tracing::{debug, trace, warn};

#[cfg(feature = "pgp")]
use crate::pgp::Pgp;
use crate::{Error, Result};

use super::{
    MULTIPART_BEGIN, MULTIPART_BEGIN_ESCAPED, MULTIPART_END, MULTIPART_END_ESCAPED, PART_BEGIN,
    PART_BEGIN_ESCAPED, PART_END, PART_END_ESCAPED,
};

/// Filters parts to show by MIME type.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum FilterParts {
    /// Show all parts. This filter enables MML markup since multiple
    /// parts with different MIME types can be mixed together, which
    /// can be hard to navigate through.
    #[default]
    All,

    /// Show only parts matching the given MIME type. This filter
    /// disables MML markup since only one MIME type is shown.
    Only(String),

    /// Show only parts matching the given list of MIME types. This
    /// filter enables MML markup since multiple parts with different
    /// MIME types can be mixed together, which can be hard to
    /// navigate through.
    Include(Vec<String>),

    /// Show all parts except those matching the given list of MIME
    /// types. This filter enables MML markup since multiple parts
    /// with different MIME types can be mixed together, which can be
    /// hard to navigate through.
    Exclude(Vec<String>),
}

impl FilterParts {
    pub fn only(&self, that: impl AsRef<str>) -> bool {
        match self {
            Self::All => false,
            Self::Only(this) => this == that.as_ref(),
            Self::Include(_) => false,
            Self::Exclude(_) => false,
        }
    }

    pub fn contains(&self, that: impl ToString + AsRef<str>) -> bool {
        match self {
            Self::All => true,
            Self::Only(this) => this == that.as_ref(),
            Self::Include(this) => this.contains(&that.to_string()),
            Self::Exclude(this) => !this.contains(&that.to_string()),
        }
    }
}

/// MIME → MML message body interpreter.
///
/// The interpreter follows the builder pattern, where the build function
/// is named `interpret_*`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MimeBodyInterpreter {
    /// Defines visibility of the multipart markup `<#multipart>`.
    ///
    /// When `true`, multipart markup is visible. This is useful when
    /// you need to see multiparts nested structure.
    ///
    /// When `false`, multipart markup is hidden. The structure is
    /// flatten, which means all parts and subparts are shown at the
    /// same top level.
    ///
    /// This option shows or hides the multipart markup, not their
    /// content. The content is always shown. To filter parts with
    /// their content, see [`MimeBodyInterpreter::filter_parts`] and
    /// [`FilterParts`].
    show_multiparts: bool,

    /// Defines visibility of the part markup `<#part>`.
    ///
    /// When `true`, part markup is visible. This is useful when you
    /// want to get more information about parts being interpreted
    /// (MIME type, description etc).
    ///
    /// When `false`, part markup is hidden. Only the content is
    /// shown.
    ///
    /// This option shows or hides the part markup, not their
    /// content. The content is always shown. To filter parts with
    /// their content, see [`MimeBodyInterpreter::filter_parts`] and
    /// [`FilterParts`].
    show_parts: bool,

    /// Defines visibility of the part markup `<#part
    /// disposition=attachment>`.
    ///
    /// This option is dedicated to attachment parts, and it overrides
    /// [`Self::show_parts`].
    show_attachments: bool,

    /// Defines visibility of the part markup `<#part
    /// disposition=inline>`.
    ///
    /// This option is dedicated to inline attachment parts, and it
    /// overrides [`Self::show_parts`].
    show_inline_attachments: bool,

    /// Defines parts visibility.
    ///
    /// This option filters parts to show or hide by their MIME
    /// type. If you want to show or hide MML markup instead, see
    /// [`Self::show_multiparts`], [`Self::show_parts`],
    /// [`Self::show_attachments`] and
    /// [`Self::show_inline_attachments`].
    filter_parts: FilterParts,

    /// Defines visibility of signatures in `text/plain` parts.
    ///
    /// When `false`, this option tries to remove signatures from
    /// plain text parts starting by the standard delimiter `-- \n`.
    show_plain_texts_signature: bool,

    /// Defines the saving strategy of attachments content.
    ///
    /// An attachment is interpreted this way: `<#part
    /// filename=attachment.ext>`.
    ///
    /// When `true`, the file (with its content) is automatically
    /// created at the given filename. Directory can be customized via
    /// [`Self::save_attachments_dir`]. This option is particularly
    /// useful when transferring a message with its attachments.
    save_attachments: bool,

    /// Defines the directory for [`Self::save_attachments`] strategy.
    ///
    /// This option saves attachments to the given directory instead
    /// of the default temporary one given by
    /// [`std::env::temp_dir()`].
    save_attachments_dir: PathBuf,

    #[cfg(feature = "pgp")]
    pgp: Option<Pgp>,
    #[cfg(feature = "pgp")]
    pgp_sender: Option<String>,
    #[cfg(feature = "pgp")]
    pgp_recipient: Option<String>,
}

impl Default for MimeBodyInterpreter {
    fn default() -> Self {
        Self {
            show_multiparts: false,
            show_parts: true,
            show_attachments: true,
            show_inline_attachments: true,
            filter_parts: Default::default(),
            show_plain_texts_signature: true,
            save_attachments: Default::default(),
            save_attachments_dir: Self::default_save_attachments_dir(),
            #[cfg(feature = "pgp")]
            pgp: Default::default(),
            #[cfg(feature = "pgp")]
            pgp_sender: Default::default(),
            #[cfg(feature = "pgp")]
            pgp_recipient: Default::default(),
        }
    }
}

impl MimeBodyInterpreter {
    pub fn default_save_attachments_dir() -> PathBuf {
        env::temp_dir()
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_show_multiparts(mut self, visibility: bool) -> Self {
        self.show_multiparts = visibility;
        self
    }

    pub fn with_show_parts(mut self, visibility: bool) -> Self {
        self.show_parts = visibility;
        self
    }

    pub fn with_filter_parts(mut self, filter: FilterParts) -> Self {
        self.filter_parts = filter;
        self
    }

    pub fn with_show_plain_texts_signature(mut self, visibility: bool) -> Self {
        self.show_plain_texts_signature = visibility;
        self
    }

    pub fn with_show_attachments(mut self, visibility: bool) -> Self {
        self.show_attachments = visibility;
        self
    }

    pub fn with_show_inline_attachments(mut self, visibility: bool) -> Self {
        self.show_inline_attachments = visibility;
        self
    }

    pub fn with_save_attachments(mut self, visibility: bool) -> Self {
        self.save_attachments = visibility;
        self
    }

    pub fn with_save_attachments_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.save_attachments_dir = dir.into();
        self
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
    pub fn with_pgp_recipient(mut self, recipient: Option<String>) -> Self {
        self.pgp_recipient = recipient;
        self
    }

    /// Replace normal opening and closing tags by escaped opening and
    /// closing tags.
    fn escape_mml_markup(text: String) -> String {
        text.replace(PART_BEGIN, PART_BEGIN_ESCAPED)
            .replace(PART_END, PART_END_ESCAPED)
            .replace(MULTIPART_BEGIN, MULTIPART_BEGIN_ESCAPED)
            .replace(MULTIPART_END, MULTIPART_END_ESCAPED)
    }

    /// Decrypt the given [MessagePart] using PGP.
    #[cfg(feature = "pgp")]
    async fn decrypt_part(&self, encrypted_part: &MessagePart<'_>) -> Result<String> {
        match &self.pgp {
            None => {
                debug!("cannot decrypt part: pgp not configured");
                Ok(String::from_utf8_lossy(encrypted_part.contents()).to_string())
            }
            Some(pgp) => {
                let recipient = self
                    .pgp_recipient
                    .as_ref()
                    .ok_or(Error::PgpDecryptMissingRecipientError)?;
                let encrypted_bytes = encrypted_part.contents().to_owned();
                let decrypted_part = pgp.decrypt(recipient, encrypted_bytes).await?;
                let clear_part = MessageParser::new()
                    .parse(&decrypted_part)
                    .ok_or(Error::ParsePgpDecryptedPartError)?;
                let tpl = self.interpret_msg(&clear_part).await?;
                Ok(tpl)
            }
        }
    }

    /// Verify the given [Message] using PGP.
    #[cfg(feature = "pgp")]
    async fn verify_msg(&self, msg: &Message<'_>, ids: &[usize]) -> Result<()> {
        match &self.pgp {
            None => {
                debug!("cannot verify message: pgp not configured");
            }
            Some(pgp) => {
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
                pgp.verify(recipient, signature_bytes, signed_part_bytes)
                    .await?;
            }
        };

        Ok(())
    }

    fn interpret_attachment(&self, ctype: &str, part: &MessagePart, data: &[u8]) -> Result<String> {
        let mut tpl = String::new();

        if self.show_attachments && self.filter_parts.contains(ctype) {
            let fname = self
                .save_attachments_dir
                .join(part.attachment_name().unwrap_or("noname"));

            if self.save_attachments {
                fs::write(&fname, data)
                    .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
            }

            let fname = fname.to_string_lossy();
            tpl = format!("<#part type={ctype} filename=\"{fname}\"><#/part>\n");
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

        if self.show_inline_attachments && self.filter_parts.contains(ctype) {
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
            tpl = format!("<#part type={ctype} disposition=inline filename=\"{fname}\"><#/part>\n");
        }

        Ok(tpl)
    }

    fn interpret_text(&self, ctype: &str, text: &str) -> String {
        let mut tpl = String::new();

        if self.filter_parts.contains(ctype) {
            let text = text.replace('\r', "");
            let text = Self::escape_mml_markup(text);

            if !self.show_parts || self.filter_parts.only(ctype) {
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
            let plain = plain.replace('\r', "");
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
                let html = html.replace('\r', "");
                let html = Self::escape_mml_markup(html);
                tpl.push_str(&html);
            } else {
                let html = html2text(&html);
                let html = Self::escape_mml_markup(html);

                if self.show_parts {
                    tpl.push_str("<#part type=text/html>\n");
                }

                tpl.push_str(&html);

                if self.show_parts {
                    tpl.push_str("<#/part>\n");
                }
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
                let mut parts = ids.iter().filter_map(|id| msg.part(*id));

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
                        match parts
                            .clone()
                            .find(|part| get_ctype(part).starts_with(ctype))
                        {
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
            #[cfg(feature = "pgp")]
            PartType::Multipart(ids) if ctype == "multipart/encrypted" => {
                match self.decrypt_part(msg.part(ids[1]).unwrap()).await {
                    Ok(ref clear_part) => tpl.push_str(clear_part),
                    Err(err) => {
                        debug!("cannot decrypt email part using pgp: {err}");
                        trace!("{err:?}");
                    }
                }
            }
            #[cfg(feature = "pgp")]
            PartType::Multipart(ids) if ctype == "multipart/signed" => {
                match self.verify_msg(msg, ids).await {
                    Ok(()) => {
                        debug!("email part successfully verified using pgp");
                    }
                    Err(err) => {
                        debug!("cannot verify email part using pgp: {err}");
                        trace!("{err:?}");
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
                        debug!("cannot find part {id}, skipping it");
                    }
                }

                if self.show_multiparts {
                    tpl.push_str("<#/multipart>\n");
                }
            }
        }

        Ok(tpl)
    }

    /// Interpret the given MIME [Message] as a MML message string.
    pub async fn interpret_msg<'a>(&self, msg: &Message<'a>) -> Result<String> {
        self.interpret_part(msg, msg.root_part()).await
    }

    /// Interpret the given MIME message bytes as a MML message
    /// string.
    pub async fn interpret_bytes<'a>(&self, bytes: impl AsRef<[u8]> + 'a) -> Result<String> {
        let msg = MessageParser::new()
            .parse(bytes.as_ref())
            .ok_or(Error::ParseMimeMessageError)?;
        self.interpret_msg(&msg).await
    }

    /// Interpret the given MIME [MessageBuilder] as a MML message
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

    use super::{FilterParts, MimeBodyInterpreter};

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

        let tpl = MimeBodyInterpreter::new()
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

        let tpl = MimeBodyInterpreter::new()
            .with_show_multiparts(true)
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

        let tpl = MimeBodyInterpreter::new()
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

        let tpl = MimeBodyInterpreter::new()
            .with_filter_parts(FilterParts::Only("text/plain".into()))
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

        let tpl = MimeBodyInterpreter::new()
            .with_filter_parts(FilterParts::Only("text/html".into()))
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

        let tpl = MimeBodyInterpreter::new()
            .with_filter_parts(FilterParts::Only("text/json".into()))
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

        let tpl = MimeBodyInterpreter::new()
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

        let tpl = MimeBodyInterpreter::new()
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

        let tpl = MimeBodyInterpreter::new()
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

        let tpl = MimeBodyInterpreter::new()
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

        let tpl = MimeBodyInterpreter::new()
            .with_filter_parts(FilterParts::Only("text/html".into()))
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

        let tpl = MimeBodyInterpreter::new()
            .with_save_attachments_dir("~/Downloads")
            .interpret_msg_builder(builder)
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "<#part type=application/octet-stream filename=\"~/Downloads/attachment.txt\"><#/part>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn hide_parts_single_html() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "text/html",
            "<h1>This is a &lt;HTML&gt; text part.</h1>\n",
        ));

        let tpl = MimeBodyInterpreter::new()
            .with_show_parts(false)
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!("This is a <HTML> text part.\r", "\r", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn hide_parts_multipart_mixed() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![
                MimePart::new("text/plain", "This is a plain text part.\n"),
                MimePart::new("text/html", "<h1>This is a &lt;HTML&gt; text part.</h1>\n"),
                MimePart::new("text/json", "{\"type\": \"This is a JSON text part.\"}\n"),
            ],
        ));

        let tpl = MimeBodyInterpreter::new()
            .with_show_parts(false)
            .with_filter_parts(FilterParts::Include(vec![
                "text/plain".into(),
                "text/html".into(),
            ]))
            .interpret_msg_builder(builder.clone())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "This is a plain text part.",
            "This is a <HTML> text part.\r",
            "\r",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }
}
