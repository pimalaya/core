use mail_builder::MessageBuilder;
use mail_parser::Message;
use pimalaya_process::Cmd;
use std::{io, path::PathBuf, result};
use thiserror::Error;

use crate::{mml, ShowAttachmentsStrategy, ShowTextsStrategy, Tpl};

use super::header;

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
    Only(Vec<String>),
}

impl ShowHeadersStrategy {
    pub fn contains(&self, header: &String) -> bool {
        match self {
            Self::All => false,
            Self::Only(headers) => headers.contains(header),
        }
    }
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
    show_headers: ShowHeadersStrategy,

    /// Inner reference to the [MML interpreter](crate::mml::Interpreter).
    mml_interpreter: mml::Interpreter,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn show_headers(mut self, s: ShowHeadersStrategy) -> Self {
        self.show_headers = s;
        self
    }

    pub fn show_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::All;
        self
    }

    pub fn show_only_headers<S: ToString, B: IntoIterator<Item = S>>(mut self, headers: B) -> Self {
        let headers = headers.into_iter().fold(Vec::new(), |mut headers, header| {
            let header = header.to_string();
            if !headers.contains(&header) {
                headers.push(header)
            }
            headers
        });
        self.show_headers = ShowHeadersStrategy::Only(headers);
        self
    }

    pub fn show_additional_headers<S: ToString, B: IntoIterator<Item = S>>(
        mut self,
        headers: B,
    ) -> Self {
        let next_headers = headers.into_iter().fold(Vec::new(), |mut headers, header| {
            let header = header.to_string();
            if !headers.contains(&header) && !self.show_headers.contains(&header) {
                headers.push(header)
            }
            headers
        });

        match &mut self.show_headers {
            ShowHeadersStrategy::All => {
                self.show_headers = ShowHeadersStrategy::Only(next_headers);
            }
            ShowHeadersStrategy::Only(headers) => {
                headers.extend(next_headers);
            }
        };

        self
    }

    pub fn hide_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::Only(Vec::new());
        self
    }

    pub fn show_multiparts(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_multiparts(b);
        self
    }

    pub fn show_texts(mut self, s: ShowTextsStrategy) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_texts(s);
        self
    }

    pub fn show_plain_texts_signature(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_plain_texts_signature(b);
        self
    }

    pub fn show_attachments(mut self, s: ShowAttachmentsStrategy) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_attachments(s);
        self
    }

    pub fn save_attachments(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.save_attachments(b);
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

        match self.show_headers {
            ShowHeadersStrategy::All => msg.headers().iter().for_each(|header| {
                let key = header.name.as_str();
                let val = header::display_value(&header.value);
                tpl.push_str(&format!("{key}: {val}\n"));
            }),
            ShowHeadersStrategy::Only(keys) => keys
                .iter()
                .filter_map(|key| msg.header(key).map(|val| (key, val)))
                .for_each(|(key, val)| {
                    let val = header::display_value(val);
                    tpl.push_str(&format!("{key}: {val}\n"));
                }),
        };

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
            "Message-ID: id@localhost",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "Content-Type: text/plain; charset=utf-8",
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
            .show_only_headers(["From", "Subject"])
            .interpret_msg_builder(msg())
            .unwrap();

        let expected_tpl = concat_line!(
            "From: from@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn only_headers_duplicated() {
        let tpl = Interpreter::new()
            .show_only_headers(["From", "Subject", "From"])
            .interpret_msg_builder(msg())
            .unwrap();

        let expected_tpl = concat_line!(
            "From: from@localhost",
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
