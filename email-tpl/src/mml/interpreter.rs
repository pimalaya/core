use log::warn;
use mail_parser::{Message, MessagePart, MimeHeaders, PartType};
use pimalaya_process::Cmd;
use std::{collections::HashSet, env, fs, io, path::PathBuf, result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    // TODO: return the original chumsky::Error
    #[error("cannot parse template: {0}")]
    ParseTplError(String),
    #[error("cannot interpret template: recipient is missing")]
    InterpretTplMissingRecipientError,
    #[error("cannot interpret template")]
    WriteInterpretdPartToVecError(#[source] io::Error),
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
    #[error("cannot save attachement content at {1}")]
    WriteAttachmentError(#[source] io::Error, PathBuf),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the show text parts strategy [`TplBuilder`] build
/// option.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ShowTextPartsStrategy {
    /// Shows plain text parts first. If none of them found, fallback
    /// to HTML.
    #[default]
    PlainOtherwiseHtml,
    /// Shows plain text parts only.
    PlainOnly,
    /// Shows HTML parts first. If none of them found, fallback to
    /// plain text.
    HtmlOtherwisePlain,
    /// Shows HTML parts only.
    HtmlOnly,
}

/// Represents the show headers [`TplBuilder`] build option.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ShowHeadersStrategy {
    /// Shows all available headers in [`TplBuilder::headers`].
    #[default]
    All,
    /// Shows only specific headers from [`TplBuilder::headers`] and
    /// overrides the order [`TplBuilder::headers_order`].
    Only(HashSet<String>),
}

impl ShowHeadersStrategy {
    pub fn all() -> Self {
        Self::All
    }

    pub fn only<I, H>(headers: H) -> Self
    where
        I: ToString,
        H: IntoIterator<Item = I>,
    {
        Self::Only(HashSet::from_iter(
            headers.into_iter().map(|h| h.to_string()),
        ))
    }

    pub fn none() -> Self {
        Self::Only(HashSet::default())
    }
}

/// Represents the interpreter builder. It allows you to customize the
/// template compilation using the [Builder pattern].
///
/// [Builder pattern]: https://en.wikipedia.org/wiki/Builder_pattern
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InterpreterBuilder {
    pub pgp_decrypt_cmd: Option<Cmd>,
    pub pgp_verify_cmd: Option<Cmd>,
    pub show_text_parts: ShowTextPartsStrategy,
    pub show_headers: ShowHeadersStrategy,
    pub show_text_parts_only: bool,
    /// Represents the build option that sanitizes text/plain parts.
    pub sanitize_text_plain_parts: bool,
    /// Represents the build option that sanitizes text/html parts.
    pub sanitize_text_html_parts: bool,
    /// Represents the build option that removes signature from
    /// text/plain parts.
    pub remove_text_plain_parts_signature: bool,

    /// If `true` then multipart structures are kept unchanged when
    /// interpreting emails as template. It is useful to see how
    /// nested parts are structured, which part is encrypted or signed
    /// etc. If `false` then multipart structure is flatten, which
    /// means all parts and subparts are shown at same the root level.
    pub show_multiparts: Option<bool>,

    /// An attachment is interpreted this way: `<#part
    /// filename=attachment.ext>`. If `true` then the file (with its
    /// content) is automatically created at the given
    /// filename. Directory can be customized via
    /// `save_attachments_dir`. This option is particularly useful
    /// when transfering an email with its attachments.
    pub save_attachments: Option<bool>,

    /// Saves attachments to the given directory instead of the
    /// default temporary one given by [`std::env::temp_dir()`].
    pub save_attachments_dir: Option<PathBuf>,
}

impl<'a> InterpreterBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_decrypt_cmd = Some(cmd.into());
        self
    }

    pub fn some_pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.pgp_decrypt_cmd = cmd.map(|c| c.into());
        self
    }

    pub fn pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_verify_cmd = Some(cmd.into());
        self
    }

    pub fn some_pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.pgp_verify_cmd = cmd.map(|c| c.into());
        self
    }

    pub fn show_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::All;
        self
    }

    /// Appends headers filters to the template builder. See
    /// [TplBuilder::show_headers] for more information about the
    /// `show_headers` build option.
    pub fn show_headers<S: ToString, B: IntoIterator<Item = S>>(mut self, headers: B) -> Self {
        let headers = headers
            .into_iter()
            .map(|header| header.to_string())
            .collect();

        match self.show_headers {
            ShowHeadersStrategy::All => {
                self.show_headers = ShowHeadersStrategy::Only(headers);
            }
            ShowHeadersStrategy::Only(prev_headers) => {
                let mut prev_headers = prev_headers.clone();
                prev_headers.extend(headers);
                self.show_headers = ShowHeadersStrategy::Only(prev_headers);
            }
        };

        self
    }

    pub fn hide_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::Only(HashSet::new());
        self
    }

    pub fn show_multiparts(mut self) -> Self {
        self.show_multiparts = Some(true);
        self
    }

    pub fn hide_multiparts(mut self) -> Self {
        self.show_multiparts = Some(false);
        self
    }

    pub fn save_attachments(mut self, b: bool) -> Self {
        self.save_attachments = Some(b);
        self
    }

    pub fn save_attachments_dir<P>(mut self, p: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.save_attachments_dir = Some(p.into());
        self
    }

    pub fn build(self) -> Interpreter {
        Interpreter {
            pgp_decrypt_cmd: self
                .pgp_decrypt_cmd
                .unwrap_or_else(|| "gpg --decrypt --quiet".into()),
            pgp_verify_cmd: self
                .pgp_verify_cmd
                .unwrap_or_else(|| "gpg --verify --quiet --recipient <recipient>".into()),
            show_text_parts: self.show_text_parts,
            show_headers: self.show_headers,
            show_text_parts_only: self.show_text_parts_only,
            sanitize_text_plain_parts: self.sanitize_text_plain_parts,
            sanitize_text_html_parts: self.sanitize_text_html_parts,
            remove_text_plain_parts_signature: self.remove_text_plain_parts_signature,
            show_multiparts: self.show_multiparts.unwrap_or(false),
            save_attachments: self.save_attachments.unwrap_or(false),
            save_attachments_dir: self.save_attachments_dir.unwrap_or_else(|| env::temp_dir()),
        }
    }
}

/// Represents the interpreter options. It is the final struct passed
/// down to the [Tpl::interpret] function.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Interpreter {
    pub pgp_decrypt_cmd: Cmd,
    pub pgp_verify_cmd: Cmd,
    pub show_text_parts: ShowTextPartsStrategy,
    pub show_headers: ShowHeadersStrategy,
    pub show_text_parts_only: bool,
    pub sanitize_text_plain_parts: bool,
    pub sanitize_text_html_parts: bool,
    pub remove_text_plain_parts_signature: bool,
    pub show_multiparts: bool,
    pub save_attachments: bool,
    pub save_attachments_dir: PathBuf,
}

impl<'a> Interpreter {
    pub fn interpret(&self, msg: &Message) -> Result<String> {
        self.interpret_part(msg, msg.root_part())
    }

    pub fn interpret_part(&self, msg: &Message, part: &MessagePart) -> Result<String> {
        let mut tpl = String::new();

        let cdisp = part.content_disposition();
        let ctype = part
            .content_type()
            .and_then(|ctype| {
                ctype
                    .subtype()
                    .map(|stype| format!("{}/{stype}", ctype.ctype()))
            })
            .unwrap_or_else(|| String::from("application/octet-stream"));

        let is_attachment = cdisp.map(|cdisp| cdisp.is_attachment()).unwrap_or(false);
        let is_inline = cdisp.map(|cdisp| cdisp.is_inline()).unwrap_or(false);

        if is_attachment {
            let fname = self
                .save_attachments_dir
                .join(part.attachment_name().unwrap_or("noname"));

            if self.save_attachments {
                fs::write(&fname, part.contents())
                    .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
            }

            let fname = fname.to_string_lossy();
            tpl.push_str(&format!("<#part filename=\"{fname}\" type=\"{ctype}\">"));
            tpl.push('\n');
        } else if is_inline {
            let fname = self
                .save_attachments_dir
                .join(part.content_id().unwrap_or("noname"));

            if self.save_attachments {
                fs::write(&fname, part.contents())
                    .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
            }

            let fname = fname.to_string_lossy();

            tpl.push_str(&format!(
                "<#part filename=\"{fname}\" type=\"{ctype}\" disposition=\"inline\">"
            ));
            tpl.push('\n');
        } else {
            match &part.body {
                PartType::Text(text) => {
                    tpl.push_str(&text);
                    tpl.push('\n');
                }
                PartType::Html(html) => {
                    tpl.push_str("<#part type=\"text/html\">");
                    tpl.push('\n');
                    tpl.push_str(&html);
                    tpl.push('\n');
                    tpl.push_str("<#/part>");
                    tpl.push('\n');
                }
                PartType::Binary(data) => {
                    let fname = self
                        .save_attachments_dir
                        .join(part.attachment_name().unwrap_or("noname"));

                    if self.save_attachments {
                        fs::write(&fname, data)
                            .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
                    }

                    let fname = fname.to_string_lossy();
                    tpl.push_str(&format!("<#part filename=\"{fname}\" type=\"{ctype}\">"));
                    tpl.push('\n');
                }
                PartType::InlineBinary(data) => {
                    let fname = self
                        .save_attachments_dir
                        .join(part.content_id().unwrap_or("noname"));

                    if self.save_attachments {
                        fs::write(&fname, data)
                            .map_err(|err| Error::WriteAttachmentError(err, fname.clone()))?;
                    }

                    let fname = fname.to_string_lossy();
                    tpl.push_str(&format!(
                        "<#part filename=\"{fname}\" type=\"{ctype}\" disposition=\"inline\">"
                    ));
                    tpl.push('\n');
                }
                PartType::Message(msg) => tpl.push_str(&self.interpret(msg)?),
                PartType::Multipart(ids) if ctype == "multipart/encrypted" => {
                    let encrypted_part = msg.part(ids[1]).unwrap();
                    let decrypted_part = self
                        .pgp_decrypt_cmd
                        .run_with(encrypted_part.text_contents().unwrap())
                        .unwrap()
                        .stdout;
                    let msg = Message::parse(&decrypted_part).unwrap();
                    tpl.push_str(&self.interpret(&msg)?);
                }
                PartType::Multipart(ids) if ctype == "multipart/signed" => {
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
                    if self.show_multiparts {
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

                    if self.show_multiparts {
                        tpl.push_str("<#/multipart>");
                        tpl.push('\n');
                    }
                }
            }
        }

        Ok(tpl)
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use mail_builder::{mime::MimePart, MessageBuilder};
    use mail_parser::Message;

    use crate::InterpreterBuilder;

    #[test]
    fn interpret_text_plain() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
            .write_to_vec()
            .unwrap();
        let msg = Message::parse(&msg).unwrap();

        let tpl = InterpreterBuilder::new().build().interpret(&msg).unwrap();
        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn interpret_text_html() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .html_body("<h1>Hello, world!</h1>")
            .write_to_vec()
            .unwrap();
        let msg = Message::parse(&msg).unwrap();

        let tpl = InterpreterBuilder::new().build().interpret(&msg).unwrap();
        let expected_tpl = concat_line!(
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn interpret_attachment() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
            .binary_attachment("text/plain", "attachment.txt", "Hello, world!".as_bytes())
            .write_to_string()
            .unwrap();
        let msg = Message::parse(msg.as_bytes()).unwrap();

        let tpl = InterpreterBuilder::new()
            .save_attachments_dir("~/Downloads")
            .build()
            .interpret(&msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "Hello, world!",
            "<#part filename=\"~/Downloads/attachment.txt\" type=\"text/plain\">",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn interpret_show_multipart() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new_multipart(
                "multipart/mixed",
                vec![
                    MimePart::new_text("Hello, world!"),
                    MimePart::new_html("<h1>Hello, world!</h1>"),
                ],
            ))
            .write_to_vec()
            .unwrap();
        let msg = Message::parse(&msg).unwrap();

        let tpl = InterpreterBuilder::new()
            .show_multiparts()
            .build()
            .interpret(&msg)
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
    fn interpret_hide_multipart() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new_multipart(
                "multipart/mixed",
                vec![
                    MimePart::new_text("Hello, world!"),
                    MimePart::new_html("<h1>Hello, world!</h1>"),
                ],
            ))
            .write_to_vec()
            .unwrap();
        let msg = Message::parse(&msg).unwrap();

        let tpl = InterpreterBuilder::new()
            .hide_multiparts()
            .build()
            .interpret(&msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "Hello, world!",
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn interpret_show_nested_multiparts() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new_multipart(
                "multipart/mixed",
                vec![MimePart::new_multipart(
                    "multipart/alternative",
                    vec![
                        MimePart::new_text("Hello, world!"),
                        MimePart::new_html("<h1>Hello, world!</h1>"),
                    ],
                )],
            ))
            .write_to_vec()
            .unwrap();
        let msg = Message::parse(&msg).unwrap();

        let tpl = InterpreterBuilder::new()
            .show_multiparts()
            .build()
            .interpret(&msg)
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

    #[test]
    fn interpret_hide_nested_multiparts() {
        let msg = MessageBuilder::new()
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .body(MimePart::new_multipart(
                "multipart/mixed",
                vec![MimePart::new_multipart(
                    "multipart/alternative",
                    vec![
                        MimePart::new_text("Hello, world!"),
                        MimePart::new_html("<h1>Hello, world!</h1>"),
                    ],
                )],
            ))
            .write_to_vec()
            .unwrap();
        let msg = Message::parse(&msg).unwrap();

        let tpl = InterpreterBuilder::new()
            .hide_multiparts()
            .build()
            .interpret(&msg)
            .unwrap();
        let expected_tpl = concat_line!(
            "Hello, world!",
            "<#part type=\"text/html\">",
            "<h1>Hello, world!</h1>",
            "<#/part>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }
}
