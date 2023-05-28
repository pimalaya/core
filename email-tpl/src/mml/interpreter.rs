use log::warn;
use mail_builder::MessageBuilder;
use mail_parser::{Message, MessagePart, MimeHeaders, PartType};
use pimalaya_process::Cmd;
use regex::Regex;
use std::{collections::HashSet, env, fs, io, path::PathBuf, result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
    #[error("cannot save attachement at {1}")]
    WriteAttachmentError(#[source] io::Error, PathBuf),
    #[error("cannot build email")]
    WriteMessageError(#[source] io::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the strategy used to display emails parts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ShowPartsStrategy {
    #[default]
    All,
    Only(HashSet<String>),
}

impl ShowPartsStrategy {
    pub fn contains<C>(&self, ctype: C) -> bool
    where
        C: AsRef<str>,
    {
        match self {
            Self::All => true,
            Self::Only(set) => set.contains(ctype.as_ref()),
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
    /// If `true` then text/plain parts are sanitized in order to be
    /// more readable: carriage returns are removed, tabulations are
    /// replaced by spaces, consecutive new line are merged and spaces
    /// are merged.
    sanitize_text_plain_parts: bool,

    /// If `true` then text/html parts are sanitized in order to be
    /// more readable: carriage returns are removed, tabulations are
    /// replaced by spaces, consecutive new line are merged and spaces
    /// are merged, HTML entities are decoded.
    sanitize_text_html_parts: bool,

    /// If `true` then tries to remove signatures for text plain
    /// parts.
    remove_text_plain_parts_signature: bool,

    /// Defines the strategy to display text parts.
    show_parts_strategy: ShowPartsStrategy,

    /// If `true` then multipart structures are kept unchanged when
    /// interpreting emails as template. It is useful to see how
    /// nested parts are structured, which part is encrypted or signed
    /// etc. If `false` then multipart structure is flatten, which
    /// means all parts and subparts are shown at same the root level.
    show_multipart_markup: bool,

    /// If `true` then part structures are kept unchanged when
    /// interpreting emails as template. It is useful to keep
    /// information about parts.
    show_part_markup: bool,

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

    /// Command used to decrypt encrypted parts.
    pgp_decrypt_cmd: Cmd,

    /// Command used to verify signed parts.
    pgp_verify_cmd: Cmd,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self {
            sanitize_text_plain_parts: false,
            sanitize_text_html_parts: false,
            remove_text_plain_parts_signature: false,
            show_parts_strategy: ShowPartsStrategy::All,
            show_multipart_markup: false,
            show_part_markup: false,
            save_attachments: false,
            save_attachments_dir: env::temp_dir(),
            pgp_decrypt_cmd: "gpg --decrypt --quiet".into(),
            pgp_verify_cmd: "gpg --verify --quiet --recipient <recipient>".into(),
        }
    }
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sanitize_text_plain_parts(mut self, b: bool) -> Self {
        self.sanitize_text_plain_parts = b;
        self
    }

    fn sanitize_plain<P>(&self, plain: P) -> String
    where
        P: ToString,
    {
        let mut plain = plain.to_string();

        // keeps a maximum of 2 consecutive new lines
        plain = Regex::new(r"(\r?\n\s*){2,}")
            .unwrap()
            .replace_all(&plain, "\n\n")
            .to_string();

        // replaces tabulations by spaces
        plain = Regex::new(r"\t")
            .unwrap()
            .replace_all(&plain, " ")
            .to_string();

        // keeps a maximum of 2 consecutive spaces
        plain = Regex::new(r" {2,}")
            .unwrap()
            .replace_all(&plain, "  ")
            .to_string();

        plain
    }

    pub fn sanitize_text_html_parts(mut self, b: bool) -> Self {
        self.sanitize_text_html_parts = b;
        self
    }

    fn sanitize_html<H>(&self, html: H) -> String
    where
        H: ToString,
    {
        let mut html = html.to_string();

        // removes html markup
        html = ammonia::Builder::new()
            .tags(HashSet::default())
            .clean(&html)
            .to_string();
        // merges new line chars
        html = Regex::new(r"(\r?\n\s*){2,}")
            .unwrap()
            .replace_all(&html, "\n\n")
            .to_string();
        // replaces tabulations and &npsp; by spaces
        html = Regex::new(r"(\t|&nbsp;)")
            .unwrap()
            .replace_all(&html, " ")
            .to_string();
        // merges spaces
        html = Regex::new(r" {2,}")
            .unwrap()
            .replace_all(&html, "  ")
            .to_string();
        // decodes html entities
        html = html_escape::decode_html_entities(&html).to_string();

        html
    }

    pub fn remove_text_plain_parts_signature(mut self) -> Self {
        self.remove_text_plain_parts_signature = true;
        self
    }

    pub fn show_all_parts(mut self) -> Self {
        self.show_parts_strategy = ShowPartsStrategy::All;
        self
    }

    pub fn show_parts<S: ToString, B: IntoIterator<Item = S>>(mut self, parts: B) -> Self {
        let parts = parts.into_iter().map(|part| part.to_string()).collect();

        match &mut self.show_parts_strategy {
            ShowPartsStrategy::All => {
                self.show_parts_strategy = ShowPartsStrategy::Only(parts);
            }
            ShowPartsStrategy::Only(prev_parts) => {
                prev_parts.extend(parts);
            }
        };

        self
    }

    pub fn show_multipart_markup(mut self) -> Self {
        self.show_multipart_markup = true;
        self
    }

    pub fn hide_multipart_markup(mut self) -> Self {
        self.show_multipart_markup = false;
        self
    }

    pub fn show_part_markup(mut self) -> Self {
        self.show_part_markup = true;
        self
    }

    pub fn hide_part_markup(mut self) -> Self {
        self.show_part_markup = false;
        self
    }

    pub fn save_attachments(mut self) -> Self {
        self.save_attachments = true;
        self
    }

    pub fn save_attachments_dir<D>(mut self, dir: D) -> Self
    where
        D: Into<PathBuf>,
    {
        self.save_attachments_dir = dir.into();
        self
    }

    pub fn pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_decrypt_cmd = cmd.into();
        self
    }

    pub fn some_pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        if let Some(cmd) = cmd {
            self.pgp_decrypt_cmd = cmd.into();
        }
        self
    }

    pub fn pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_verify_cmd = cmd.into();
        self
    }

    pub fn some_pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        if let Some(cmd) = cmd {
            self.pgp_verify_cmd = cmd.into();
        }
        self
    }

    fn interpret_part(&self, msg: &Message, part: &MessagePart) -> Result<String> {
        let mut tpl = String::new();

        let ctype = part
            .content_type()
            .and_then(|ctype| {
                ctype
                    .subtype()
                    .map(|stype| format!("{}/{stype}", ctype.ctype()))
            })
            .unwrap_or_else(|| String::from("application/octet-stream"));

        let cdisp = part.content_disposition();

        let is_attachment = cdisp.map(|cdisp| cdisp.is_attachment()).unwrap_or(false);
        let is_inline = cdisp.map(|cdisp| cdisp.is_inline()).unwrap_or(false);

        if is_attachment {
            if self.show_parts_strategy.contains(&ctype) {
                let fname = self
                    .save_attachments_dir
                    .join(part.attachment_name().unwrap_or("noname"));

                if self.save_attachments {
                    fs::write(&fname, part.contents())
                        .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
                }

                if self.show_part_markup {
                    let fname = fname.to_string_lossy();
                    tpl.push_str(&format!("<#part filename=\"{fname}\" type=\"{ctype}\">"));
                    tpl.push('\n');
                }
            }
        } else if is_inline {
            if self.show_parts_strategy.contains(&ctype) {
                let fname = self
                    .save_attachments_dir
                    .join(part.content_id().unwrap_or("noname"));

                if self.save_attachments {
                    fs::write(&fname, part.contents())
                        .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
                }

                if self.show_part_markup {
                    let fname = fname.to_string_lossy();
                    tpl.push_str(&format!(
                        "<#part filename=\"{fname}\" type=\"{ctype}\" disposition=\"inline\">"
                    ));
                    tpl.push('\n');
                }
            }
        } else {
            match &part.body {
                PartType::Text(plain) => {
                    if self.show_parts_strategy.contains(&ctype) {
                        let mut plain = plain.to_string();

                        if self.remove_text_plain_parts_signature {
                            plain = plain
                                .rsplit_once("-- \n")
                                .map(|(body, _signature)| body.to_owned())
                                .unwrap_or(plain);
                        }

                        if self.sanitize_text_plain_parts {
                            plain = self.sanitize_plain(plain);
                        }

                        tpl.push_str(&plain.trim());
                        tpl.push('\n');
                    }
                }
                PartType::Html(html) => {
                    if self.show_parts_strategy.contains(&ctype) {
                        let mut html = html.to_string();

                        if self.show_part_markup {
                            tpl.push_str("<#part type=\"text/html\">");
                            tpl.push('\n');
                        }

                        if self.sanitize_text_html_parts {
                            html = self.sanitize_html(html);
                        }

                        tpl.push_str(&html.trim());
                        tpl.push('\n');

                        if self.show_part_markup {
                            tpl.push_str("<#/part>");
                            tpl.push('\n');
                        }
                    }
                }
                PartType::Binary(data) => {
                    if self.show_parts_strategy.contains(&ctype) {
                        let fname = self
                            .save_attachments_dir
                            .join(part.attachment_name().unwrap_or("noname"));

                        if self.save_attachments {
                            fs::write(&fname, data)
                                .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
                        }

                        if self.show_part_markup {
                            let fname = fname.to_string_lossy();
                            tpl.push_str(&format!("<#part filename=\"{fname}\" type=\"{ctype}\">"));
                            tpl.push('\n');
                        }
                    }
                }
                PartType::InlineBinary(data) => {
                    if self.show_parts_strategy.contains(&ctype) {
                        let fname = self
                            .save_attachments_dir
                            .join(part.content_id().unwrap_or("noname"));

                        if self.save_attachments {
                            fs::write(&fname, data)
                                .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
                        }

                        if self.show_part_markup {
                            let fname = fname.to_string_lossy();
                            tpl.push_str(&format!(
                            "<#part filename=\"{fname}\" type=\"{ctype}\" disposition=\"inline\">"
                        ));
                            tpl.push('\n');
                        }
                    }
                }
                PartType::Message(msg) => tpl.push_str(&self.interpret_msg(msg)?),
                PartType::Multipart(ids) if ctype == "multipart/encrypted" => {
                    // TODO: clean me
                    let encrypted_part = msg.part(ids[1]).unwrap();
                    let decrypted_part = self
                        .pgp_decrypt_cmd
                        .run_with(encrypted_part.text_contents().unwrap())
                        .unwrap()
                        .stdout;
                    println!("decrypted_part: {:?}", decrypted_part);
                    let msg = Message::parse(&decrypted_part).unwrap();
                    tpl.push_str(&self.interpret_msg(&msg)?);
                }
                PartType::Multipart(ids) if ctype == "multipart/signed" => {
                    // TODO: clean me
                    let signed_part = msg.part(ids[0]).unwrap();
                    let signature_part = msg.part(ids[1]).unwrap();
                    self.pgp_verify_cmd
                        .run_with(signature_part.text_contents().unwrap())
                        .unwrap();
                    tpl.push_str(&self.interpret_part(&msg, signed_part)?);
                }
                PartType::Multipart(_) if ctype == "application/pgp-encrypted" => (),
                PartType::Multipart(_) if ctype == "application/pgp-signature" => (),
                PartType::Multipart(ids) => {
                    if self.show_multipart_markup {
                        let stype = part
                            .content_type()
                            .and_then(|p| p.subtype())
                            .unwrap_or("mixed");
                        tpl.push_str(&format!("<#multipart type=\"{stype}\">"));
                        tpl.push('\n');
                    }

                    for id in ids {
                        if let Some(part) = msg.part(*id) {
                            tpl.push_str(&self.interpret_part(msg, part)?);
                        } else {
                            warn!("cannot find part {id}, skipping it");
                        }
                    }

                    if self.show_multipart_markup {
                        tpl.push_str("<#/multipart>");
                        tpl.push('\n');
                    }
                }
            }
        }

        Ok(tpl)
    }

    /// Interprets the given [`mail_parser::Message`] as a MML string.
    pub fn interpret_msg(&self, msg: &Message) -> Result<String> {
        self.interpret_part(msg, msg.root_part())
    }

    /// Interprets the given bytes as a MML string.
    pub fn interpret_bytes<B: AsRef<[u8]>>(&self, bytes: B) -> Result<String> {
        let msg = Message::parse(bytes.as_ref()).ok_or(Error::ParseRawEmailError)?;
        self.interpret_msg(&msg)
    }

    /// Interprets the given [`mail_builder::MessageBuilder`] as a MML
    /// string.
    pub fn interpret_msg_builder(&self, builder: MessageBuilder) -> Result<String> {
        let bytes = builder.write_to_vec().map_err(Error::WriteMessageError)?;
        self.interpret_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use mail_builder::{mime::MimePart, MessageBuilder};

    use super::Interpreter;

    #[test]
    fn plain() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!");
        let tpl = Interpreter::new().interpret_msg_builder(msg).unwrap();
        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn html() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .html_body("<h1>Hello, world!</h1>");
        let tpl = Interpreter::new().interpret_msg_builder(msg).unwrap();
        let expected_tpl = concat_line!("<h1>Hello, world!</h1>", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn html_with_markup() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .html_body("<h1>Hello, world!</h1>");
        let tpl = Interpreter::new()
            .show_part_markup()
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn html_with_sanitize() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .html_body("<h1>Hello, world!</h1>");
        let tpl = Interpreter::new()
            .sanitize_text_html_parts()
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn attachment() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
            .attachment("text/plain", "attachment.txt", "Hello, world!".as_bytes());
        let tpl = Interpreter::new()
            .save_attachments_dir("~/Downloads")
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn attachment_with_markup() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
            .attachment("text/plain", "attachment.txt", "Hello, world!".as_bytes());
        let tpl = Interpreter::new()
            .show_part_markup()
            .save_attachments_dir("~/Downloads")
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "Hello, world!",
            "<#part filename=\"~/Downloads/attachment.txt\" type=\"text/plain\">",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn multipart() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new(
                "multipart/mixed",
                vec![
                    MimePart::new("text/plain", "Hello, world!"),
                    MimePart::new("text/html", "<h1>Hello, world!</h1>"),
                ],
            ));
        let tpl = Interpreter::new()
            .hide_multipart_markup()
            .hide_part_markup()
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!("Hello, world!", "<h1>Hello, world!</h1>", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn multipart_plain_only() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new(
                "multipart/alternative",
                vec![
                    MimePart::new("text/plain", "Hello, world!"),
                    MimePart::new("text/html", "<h1>Hello, world!</h1>"),
                ],
            ));
        let tpl = Interpreter::new()
            .hide_multipart_markup()
            .hide_part_markup()
            .show_parts(["text/plain"])
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn multipart_with_markups() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new(
                "multipart/mixed",
                vec![
                    MimePart::new("text/plain", "Hello, world!"),
                    MimePart::new("text/html", "<h1>Hello, world!</h1>"),
                ],
            ));
        let tpl = Interpreter::new()
            .show_multipart_markup()
            .show_part_markup()
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "<#multipart type=\"mixed\">",
            "Hello, world!",
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
            "<#/multipart>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn nested_multipart() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new(
                "multipart/mixed",
                vec![MimePart::new(
                    "multipart/alternative",
                    vec![
                        MimePart::new("text/plain", "Hello, world!"),
                        MimePart::new("text/html", "<h1>Hello, world!</h1>"),
                    ],
                )],
            ));
        let tpl = Interpreter::new()
            .hide_multipart_markup()
            .hide_part_markup()
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!("Hello, world!", "<h1>Hello, world!</h1>", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn nested_multipart_with_markups() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new(
                "multipart/mixed",
                vec![MimePart::new(
                    "multipart/alternative",
                    vec![
                        MimePart::new("text/plain", "Hello, world!"),
                        MimePart::new("text/html", "<h1>Hello, world!</h1>"),
                    ],
                )],
            ));
        let tpl = Interpreter::new()
            .show_multipart_markup()
            .show_part_markup()
            .interpret_msg_builder(msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "<#multipart type=\"mixed\">",
            "<#multipart type=\"alternative\">",
            "Hello, world!",
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
            "<#/multipart>",
            "<#/multipart>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }
}
