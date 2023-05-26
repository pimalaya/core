use mail_builder::MessageBuilder;
use mail_parser::Message;
use pimalaya_process::Cmd;
use std::{collections::HashSet, io, path::PathBuf, result};
use thiserror::Error;

use crate::{mml, Tpl};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
    #[error("cannot build email")]
    BuildEmailError(#[source] io::Error),
    #[error("cannot interpret email body as mml")]
    InterpretMmlError(#[source] mml::interpreter::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// Represents the strategy used to display headers when interpreting
/// emails.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ShowHeadersStrategy {
    /// Transfers all available headers to the interpreted template.
    #[default]
    All,
    /// Transfers only specific headers to the interpreted template.
    Only(HashSet<String>),
}

/// The template interpreter interprets full emails as
/// [`crate::Tpl`]. The interpreter needs to be customized first. The
/// customization follows the builder pattern. When the interpreter is
/// customized, calling any function matching `interpret_*()` consumes
/// the interpreter and generates the final [`crate::Tpl`].
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Interpreter {
    /// Defines the strategy to display headers.
    /// [`ShowHeadersStrategy::All`] transfers all the available
    /// headers to the interpreted template,
    /// [`ShowHeadersStrategy::Only`] only transfers the given headers
    /// to the interpreted template.
    show_headers_strategy: ShowHeadersStrategy,

    additional_headers: Vec<(String, String)>,

    /// Inner reference to the [MML interpreter](crate::mml::Interpreter).
    mml_interpreter: mml::Interpreter,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sanitize_text_plain_parts(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.sanitize_text_plain_parts();
        self
    }

    pub fn sanitize_text_html_parts(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.sanitize_text_html_parts();
        self
    }

    pub fn sanitize_text_parts(mut self) -> Self {
        self = self.sanitize_text_plain_parts().sanitize_text_html_parts();
        self
    }

    pub fn remove_text_plain_parts_signature(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.remove_text_plain_parts_signature();
        self
    }

    pub fn show_all_parts(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_all_parts();
        self
    }

    pub fn show_parts<S: ToString, P: IntoIterator<Item = S>>(mut self, parts: P) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_parts(parts);
        self
    }

    pub fn show_all_headers(mut self) -> Self {
        self.show_headers_strategy = ShowHeadersStrategy::All;
        self
    }

    pub fn show_headers<S: ToString, B: IntoIterator<Item = S>>(mut self, headers: B) -> Self {
        let headers = headers
            .into_iter()
            .map(|header| header.to_string())
            .collect();

        match self.show_headers_strategy {
            ShowHeadersStrategy::All => {
                self.show_headers_strategy = ShowHeadersStrategy::Only(headers);
            }
            ShowHeadersStrategy::Only(prev_headers) => {
                let mut prev_headers = prev_headers.clone();
                prev_headers.extend(headers);
                self.show_headers_strategy = ShowHeadersStrategy::Only(prev_headers);
            }
        };

        self
    }

    pub fn hide_all_headers(mut self) -> Self {
        self.show_headers_strategy = ShowHeadersStrategy::Only(HashSet::new());
        self
    }

    pub fn show_multipart_markup(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_multipart_markup();
        self
    }

    pub fn hide_multipart_markup(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.hide_multipart_markup();
        self
    }

    pub fn show_part_markup(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_part_markup();
        self
    }

    pub fn hide_part_markup(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.hide_part_markup();
        self
    }

    pub fn save_attachments(mut self) -> Self {
        self.mml_interpreter = self.mml_interpreter.save_attachments();
        self
    }

    pub fn save_attachments_dir<D>(mut self, dir: D) -> Self
    where
        D: Into<PathBuf>,
    {
        self.mml_interpreter = self.mml_interpreter.save_attachments_dir(dir);
        self
    }

    pub fn pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.mml_interpreter = self.mml_interpreter.pgp_decrypt_cmd(cmd);
        self
    }

    pub fn some_pgp_decrypt_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.mml_interpreter = self.mml_interpreter.some_pgp_decrypt_cmd(cmd);
        self
    }

    pub fn pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.mml_interpreter = self.mml_interpreter.pgp_verify_cmd(cmd);
        self
    }

    pub fn some_pgp_verify_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.mml_interpreter = self.mml_interpreter.some_pgp_verify_cmd(cmd);
        self
    }

    /// Interprets the given [`mail_parser::Message`] as a
    /// [`crate::Tpl`].
    pub fn interpret_msg(self, msg: &Message) -> Result<Tpl> {
        let mut tpl = Tpl::new();

        for (key, val) in msg.headers_raw() {
            let key = key.trim();
            let val = val.trim();

            match self.show_headers_strategy {
                ShowHeadersStrategy::All => tpl.push_str(&format!("{key}: {val}\n")),
                ShowHeadersStrategy::Only(ref keys) if keys.contains(key) => {
                    tpl.push_str(&format!("{key}: {val}\n"))
                }
                ShowHeadersStrategy::Only(_) => (),
            }
        }

        if !tpl.is_empty() {
            tpl.push_str("\n");
        }

        let mml = self
            .mml_interpreter
            .interpret_msg(msg)
            .map_err(Error::InterpretMmlError)?;

        tpl.push_str(&mml);

        Ok(tpl)
    }

    /// Interprets the given bytes as a [`crate::Tpl`].
    pub fn interpret_bytes<B: AsRef<[u8]>>(self, bytes: B) -> Result<Tpl> {
        let msg = Message::parse(bytes.as_ref()).ok_or(Error::ParseRawEmailError)?;
        self.interpret_msg(&msg)
    }

    /// Interprets the given [`mail_builder::MessageBuilder`] as a
    /// [`crate::Tpl`].
    pub fn interpret_msg_builder(self, builder: MessageBuilder) -> Result<Tpl> {
        let bytes = builder.write_to_vec().map_err(Error::BuildEmailError)?;
        self.interpret_bytes(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use mail_builder::MessageBuilder;

    use super::Interpreter;

    fn msg() -> MessageBuilder<'static> {
        MessageBuilder::new()
            .message_id("id@localhost")
            .date(0 as u64)
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
    }

    #[test]
    fn all_headers() {
        let tpl = Interpreter::new()
            .show_all_headers()
            .interpret_msg_builder(msg())
            .unwrap();

        let expected_tpl = concat_line!(
            "Message-ID: <id@localhost>",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000",
            "From: <from@localhost>",
            "To: <to@localhost>",
            "Subject: subject",
            "Content-Type: text/plain; charset=\"utf-8\"",
            "Content-Transfer-Encoding: 7bit",
            "",
            "Hello, world!",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn only_headers() {
        let tpl = Interpreter::new()
            .show_headers(["From", "Subject"])
            .interpret_msg_builder(msg())
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <from@localhost>",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn no_headers() {
        let tpl = Interpreter::new()
            .hide_all_headers()
            .interpret_msg_builder(msg())
            .unwrap();

        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(*tpl, expected_tpl);
    }
}
