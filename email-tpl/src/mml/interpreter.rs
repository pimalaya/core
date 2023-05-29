use log::warn;
use mail_builder::MessageBuilder;
use mail_parser::{
    decoders::html::{html_to_text, text_to_html},
    Message, MessagePart, MimeHeaders, PartType,
};
use pimalaya_process::Cmd;
use std::{env, fs, io, path::PathBuf, result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
    #[error("cannot save attachement at {1}")]
    WriteAttachmentError(#[source] io::Error, PathBuf),
    #[error("cannot build email")]
    WriteMessageError(#[source] io::Error),
    #[error("cannot decrypt email part")]
    DecryptPartError(#[source] pimalaya_process::Error),
    #[error("cannot verify email part")]
    VerifyPartError(#[source] pimalaya_process::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the strategy used to display text parts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ShowTextsStrategy {
    /// Shows all `text/*` parts.
    All,
    /// Shows all `text/*` parts except `text/html` ones.
    AllExceptHtml,
    /// Shows only `text/plain` parts.
    #[default]
    Plain,
    /// Shows only `text/plain` parts converted to HTML (content
    /// wrapped into `<html><body></body></html>`).
    PlainConvertedToHtml,
    /// Shows only `text/html` parts.
    Html,
    /// Shows only `text/html` parts converted to plain (all the
    /// markup is removed).
    HtmlConvertedToPlain,
    /// Hides all text parts.
    None,
}

impl ShowTextsStrategy {
    pub fn allow_plain(&self) -> bool {
        match self {
            Self::All => true,
            Self::AllExceptHtml => true,
            Self::Plain => true,
            Self::PlainConvertedToHtml => true,
            Self::Html => false,
            Self::HtmlConvertedToPlain => false,
            Self::None => false,
        }
    }

    pub fn allow_html(&self) -> bool {
        match self {
            Self::All => true,
            Self::AllExceptHtml => true,
            Self::Plain => false,
            Self::PlainConvertedToHtml => false,
            Self::Html => true,
            Self::HtmlConvertedToPlain => true,
            Self::None => false,
        }
    }

    pub fn allow_conversion(&self) -> bool {
        match self {
            Self::All => false,
            Self::AllExceptHtml => false,
            Self::Plain => false,
            Self::PlainConvertedToHtml => true,
            Self::Html => false,
            Self::HtmlConvertedToPlain => true,
            Self::None => false,
        }
    }

    pub fn allow_mml_markup(&self) -> bool {
        match self {
            Self::All => true,
            Self::AllExceptHtml => true,
            Self::Plain => false,
            Self::PlainConvertedToHtml => false,
            Self::Html => false,
            Self::HtmlConvertedToPlain => false,
            Self::None => false,
        }
    }
}

/// Represents the strategy used to display attachments.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ShowAttachmentsStrategy {
    /// Shows all kind of attachments.
    #[default]
    All,
    /// Shows only attachments.
    Attachment,
    /// Shows only inline attachments.
    Inline,
    /// Hides all attachments.
    None,
}

impl ShowAttachmentsStrategy {
    pub fn allow_attachment(&self) -> bool {
        match self {
            Self::All => true,
            Self::Attachment => true,
            Self::Inline => false,
            Self::None => false,
        }
    }

    pub fn allow_inline(&self) -> bool {
        match self {
            Self::All => true,
            Self::Attachment => false,
            Self::Inline => true,
            Self::None => false,
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

    /// Represents the strategy used to display text parts.
    show_texts: ShowTextsStrategy,

    /// If `false` then tries to remove signatures for text plain
    /// parts starting by the standard delimiter `-- \n`.
    show_plain_texts_signature: bool,

    /// Represents the strategy used to display attachments.
    show_attachments: ShowAttachmentsStrategy,

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
            show_multiparts: false,
            show_texts: ShowTextsStrategy::default(),
            show_plain_texts_signature: true,
            show_attachments: ShowAttachmentsStrategy::default(),
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

    pub fn show_multiparts(mut self, b: bool) -> Self {
        self.show_multiparts = b;
        self
    }

    pub fn show_texts(mut self, s: ShowTextsStrategy) -> Self {
        self.show_texts = s;
        self
    }

    pub fn show_plain_texts_signature(mut self, b: bool) -> Self {
        self.show_plain_texts_signature = b;
        self
    }

    pub fn show_attachments(mut self, s: ShowAttachmentsStrategy) -> Self {
        self.show_attachments = s;
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

        if is_attachment && self.show_attachments.allow_attachment() {
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

            return Ok(tpl);
        }

        match &part.body {
            PartType::Text(plain) => {
                if self.show_texts.allow_plain() {
                    let mut plain = plain.replace("\r", "");

                    if !self.show_plain_texts_signature {
                        plain = plain
                            .rsplit_once("-- \n")
                            .map(|(body, _signature)| body.to_owned())
                            .unwrap_or(plain);
                    }

                    if self.show_texts.allow_conversion() {
                        plain = text_to_html(&plain);
                    }

                    if self.show_texts.allow_mml_markup() {
                        tpl.push_str(&format!("<#part type=\"{ctype}\">"));
                        tpl.push('\n');
                    }

                    tpl.push_str(&plain);
                    tpl.push('\n');

                    if self.show_texts.allow_mml_markup() {
                        tpl.push_str(&format!("<#/part>"));
                        tpl.push('\n');
                    }
                }
            }
            PartType::Html(html) => {
                if self.show_texts.allow_html() {
                    let mut html = html.replace("\r", "");

                    if self.show_texts.allow_conversion() {
                        html = html_to_text(&html);
                    }

                    if self.show_texts.allow_mml_markup() {
                        tpl.push_str(&format!("<#part type=\"{ctype}\">"));
                        tpl.push('\n');
                    }

                    tpl.push_str(&html);
                    tpl.push('\n');

                    if self.show_texts.allow_mml_markup() {
                        tpl.push_str(&format!("<#/part>"));
                        tpl.push('\n');
                    }
                }
            }
            PartType::Binary(data) => {
                if self.show_attachments.allow_attachment() {
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
            }
            PartType::InlineBinary(data) => {
                if self.show_attachments.allow_inline() {
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
            }
            PartType::Message(msg) => tpl.push_str(&self.interpret_msg(msg)?),
            PartType::Multipart(ids) if ctype == "multipart/encrypted" => {
                let encrypted_part = msg.part(ids[1]).unwrap();
                let decrypted_part = self
                    .pgp_decrypt_cmd
                    .run_with(encrypted_part.contents())
                    .map_err(Error::DecryptPartError)?
                    .stdout;
                let msg = Message::parse(&decrypted_part).unwrap();
                tpl.push_str(&self.interpret_msg(&msg)?);
            }
            PartType::Multipart(ids) if ctype == "multipart/signed" => {
                let signed_part = msg.part(ids[0]).unwrap();
                let signature_part = msg.part(ids[1]).unwrap();
                self.pgp_verify_cmd
                    .run_with(signature_part.contents())
                    .map_err(Error::VerifyPartError)?;
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
    fn show_nested_multiparts() {
        let builder = MessageBuilder::new().body(MimePart::new(
            "multipart/mixed",
            vec![MimePart::new(
                "multipart/alternative",
                vec![
                    MimePart::new("text/plain", "This is a plain text part."),
                    MimePart::new("text/html", "<h1>This is a HTML text part.</h1>"),
                ],
            )],
        ));

        let tpl = Interpreter::new()
            .show_multiparts(false)
            .interpret_msg_builder(builder.clone())
            .unwrap();

        let expected_tpl = concat_line!("This is a plain text part.", "");

        assert_eq!(tpl, expected_tpl);

        let tpl = Interpreter::new()
            .show_multiparts(true)
            .interpret_msg_builder(builder)
            .unwrap();

        let expected_tpl = concat_line!(
            "<#multipart type=\"mixed\">",
            "<#multipart type=\"alternative\">",
            "This is a plain text part.",
            "<#/multipart>",
            "<#/multipart>",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[test]
    fn plain() {
        let msg = MessageBuilder::new().text_body("Hello, world!");
        let tpl = Interpreter::new().interpret_msg_builder(msg).unwrap();
        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(tpl, expected_tpl);
    }

    // #[test]
    // fn html() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .html_body("<h1>Hello, world!</h1>");

    //     let tpl = Interpreter::new().interpret_msg_builder(msg).unwrap();

    //     let expected_tpl = concat_line!("<h1>Hello, world!</h1>", "");

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn html_with_markup() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .html_body("<h1>Hello, world!</h1>");

    //     let tpl = Interpreter::new()
    //         .show_mml_part_markup(true)
    //         .interpret_msg_builder(msg)
    //         .unwrap();

    //     let expected_tpl = concat_line!(
    //         "<#part type=\"text/html\">",
    //         "<h1>Hello, world!</h1>",
    //         "<#/part>",
    //         "",
    //     );

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn attachment() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .text_body("Hello, world!")
    //         .attachment("text/plain", "attachment.txt", "Hello, world!".as_bytes());
    //     let tpl = Interpreter::new()
    //         .save_attachments_dir("~/Downloads")
    //         .interpret_msg_builder(msg)
    //         .unwrap();
    //     let expected_tpl = concat_line!("Hello, world!", "");

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn attachment_with_markup() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .text_body("Hello, world!")
    //         .attachment("text/plain", "attachment.txt", "Hello, world!".as_bytes());
    //     let tpl = Interpreter::new()
    //         .show_mml_part_markup(true)
    //         .save_attachments_dir("~/Downloads")
    //         .interpret_msg_builder(msg)
    //         .unwrap();
    //     let expected_tpl = concat_line!(
    //         "Hello, world!",
    //         "<#part filename=\"~/Downloads/attachment.txt\" type=\"text/plain\">",
    //         "",
    //     );

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn show_multiparts() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .body(MimePart::new(
    //             "multipart/mixed",
    //             vec![
    //                 MimePart::new("text/plain", "Hello, world!"),
    //                 MimePart::new("text/html", "<h1>Hello, world!</h1>"),
    //             ],
    //         ));
    //     let tpl = Interpreter::new()
    //         .hide_mml_markup()
    //         .interpret_msg_builder(msg)
    //         .unwrap();
    //     let expected_tpl = concat_line!("Hello, world!", "<h1>Hello, world!</h1>", "");

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn multipart_plain_only() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .body(MimePart::new(
    //             "multipart/alternative",
    //             vec![
    //                 MimePart::new("text/plain", "Hello, world!"),
    //                 MimePart::new("text/html", "<h1>Hello, world!</h1>"),
    //             ],
    //         ));
    //     let tpl = Interpreter::new()
    //         .hide_mml_markup()
    //         .show_only_parts(["text/plain"])
    //         .interpret_msg_builder(msg)
    //         .unwrap();
    //     let expected_tpl = concat_line!("Hello, world!", "");

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn multipart_with_markups() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .body(MimePart::new(
    //             "multipart/mixed",
    //             vec![
    //                 MimePart::new("text/plain", "Hello, world!"),
    //                 MimePart::new("text/html", "<h1>Hello, world!</h1>"),
    //             ],
    //         ));
    //     let tpl = Interpreter::new()
    //         .show_mml_markup(true)
    //         .interpret_msg_builder(msg)
    //         .unwrap();
    //     let expected_tpl = concat_line!(
    //         "<#multipart type=\"mixed\">",
    //         "Hello, world!",
    //         "<#part type=\"text/html\">",
    //         "<h1>Hello, world!</h1>",
    //         "<#/part>",
    //         "<#/multipart>",
    //         "",
    //     );

    //     assert_eq!(tpl, expected_tpl);
    // }

    // #[test]
    // fn nested_multipart() {
    //     let msg = MessageBuilder::new()
    //         .from("from@localhost")
    //         .to("to@localhost")
    //         .subject("subject")
    //         .body(MimePart::new(
    //             "multipart/mixed",
    //             vec![MimePart::new(
    //                 "multipart/alternative",
    //                 vec![
    //                     MimePart::new("text/plain", "Hello, world!"),
    //                     MimePart::new("text/html", "<h1>Hello, world!</h1>"),
    //                 ],
    //             )],
    //         ));
    //     let tpl = Interpreter::new()
    //         .hide_mml_markup()
    //         .interpret_msg_builder(msg)
    //         .unwrap();
    //     let expected_tpl = concat_line!("Hello, world!", "<h1>Hello, world!</h1>", "");

    //     assert_eq!(tpl, expected_tpl);
    // }
}
