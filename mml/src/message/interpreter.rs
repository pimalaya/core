//! # MIME to MML message interpretation module
//!
//! Module dedicated to MIME → MML message interpretation.

use mail_builder::MessageBuilder;
use mail_parser::{Message, MessageParser};
use std::path::PathBuf;

#[cfg(feature = "pgp")]
use crate::pgp::Pgp;
use crate::{
    message::{FilterParts, MimeBodyInterpreter},
    Error, Result,
};

use super::header;

/// Filters headers to show in the interpreted message.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum FilterHeaders {
    /// Include all available headers to the interpreted message.
    #[default]
    All,

    /// Include given headers to the interpreted message.
    Include(Vec<String>),

    /// Exclude given headers from the interpreted message.
    Exclude(Vec<String>),
}

impl FilterHeaders {
    pub fn contains(&self, header: &String) -> bool {
        match self {
            Self::All => false,
            Self::Include(headers) => headers.contains(header),
            Self::Exclude(headers) => !headers.contains(header),
        }
    }
}

/// MIME → MML message interpreter builder.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MimeInterpreterBuilder {
    /// The strategy to display headers.
    show_headers: FilterHeaders,

    /// The internal MIME to MML message body interpreter.
    mime_body_interpreter: MimeBodyInterpreter,
}

impl MimeInterpreterBuilder {
    /// Create a new interpreter builder with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter headers with the given strategy.
    pub fn with_show_headers(mut self, s: FilterHeaders) -> Self {
        self.show_headers = s;
        self
    }

    /// Show all headers.
    pub fn with_show_all_headers(mut self) -> Self {
        self.show_headers = FilterHeaders::All;
        self
    }

    /// Show only headers matching the given ones.
    pub fn with_show_only_headers(
        mut self,
        headers: impl IntoIterator<Item = impl ToString>,
    ) -> Self {
        let headers = headers.into_iter().fold(Vec::new(), |mut headers, header| {
            let header = header.to_string();
            if !headers.contains(&header) {
                headers.push(header)
            }
            headers
        });
        self.show_headers = FilterHeaders::Include(headers);
        self
    }

    /// Show additional headers.
    // FIXME: seems not to work as expected, maybe need to use a
    // different structure than [FilterHeaders].
    pub fn with_show_additional_headers(
        mut self,
        headers: impl IntoIterator<Item = impl ToString>,
    ) -> Self {
        let next_headers = headers.into_iter().fold(Vec::new(), |mut headers, header| {
            let header = header.to_string();
            if !headers.contains(&header) && !self.show_headers.contains(&header) {
                headers.push(header)
            }
            headers
        });

        match &mut self.show_headers {
            FilterHeaders::All => {
                // FIXME: this excludes all previous headers, needs to
                // be separated.
                self.show_headers = FilterHeaders::Include(next_headers);
            }
            FilterHeaders::Include(headers) => {
                headers.extend(next_headers);
            }
            FilterHeaders::Exclude(headers) => {
                headers.extend(next_headers);
            }
        };

        self
    }

    /// Hide all headers.
    pub fn with_hide_all_headers(mut self) -> Self {
        self.show_headers = FilterHeaders::Include(Vec::new());
        self
    }

    /// Show MML multipart tags.
    pub fn with_show_multiparts(mut self, b: bool) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_show_multiparts(b);
        self
    }

    /// Show MML parts tags.
    pub fn with_show_parts(mut self, visibility: bool) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_show_parts(visibility);
        self
    }

    /// Filter parts using the given strategy.
    pub fn with_filter_parts(mut self, f: FilterParts) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_filter_parts(f);
        self
    }

    /// Show plain texts signature.
    pub fn with_show_plain_texts_signature(mut self, b: bool) -> Self {
        self.mime_body_interpreter = self
            .mime_body_interpreter
            .with_show_plain_texts_signature(b);
        self
    }

    /// Show MML attachments tags.
    pub fn with_show_attachments(mut self, b: bool) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_show_attachments(b);
        self
    }

    /// Show MML inline attachments tags.
    pub fn with_show_inline_attachments(mut self, b: bool) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_show_inline_attachments(b);
        self
    }

    /// Automatically save attachments to the `save_attachments_dir`.
    pub fn with_save_attachments(mut self, b: bool) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_save_attachments(b);
        self
    }

    /// Customize the download attachments directory.
    ///
    /// This can be used to display the `filename` property but also
    /// to automatically save attachment with `save_attachments`.
    pub fn with_save_attachments_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.mime_body_interpreter = self.mime_body_interpreter.with_save_attachments_dir(dir);
        self
    }

    /// Customize the download attachments directory using an optional
    /// path.
    ///
    /// This can be used to display the `filename` property but also
    /// to automatically save attachment with `save_attachments`.
    pub fn with_save_some_attachments_dir(self, dir: Option<impl Into<PathBuf>>) -> Self {
        match dir {
            Some(dir) => self.with_save_attachments_dir(dir),
            None => {
                self.with_save_attachments_dir(MimeBodyInterpreter::default_save_attachments_dir())
            }
        }
    }

    /// Customize PGP.
    #[cfg(feature = "pgp")]
    pub fn set_pgp(&mut self, pgp: impl Into<Pgp>) {
        self.mime_body_interpreter.set_pgp(pgp);
    }

    /// Customize PGP.
    #[cfg(feature = "pgp")]
    pub fn with_pgp(mut self, pgp: impl Into<Pgp>) -> Self {
        self.mime_body_interpreter.set_pgp(pgp);
        self
    }

    /// Customize some PGP.
    #[cfg(feature = "pgp")]
    pub fn set_some_pgp(&mut self, pgp: Option<impl Into<Pgp>>) {
        self.mime_body_interpreter.set_some_pgp(pgp);
    }

    /// Customize some PGP.
    #[cfg(feature = "pgp")]
    pub fn with_some_pgp(mut self, pgp: Option<impl Into<Pgp>>) -> Self {
        self.mime_body_interpreter.set_some_pgp(pgp);
        self
    }

    /// Build the final [MimeInterpreter].
    ///
    /// This intermediate step is not necessary for the interpreter,
    /// the aim is just to have a common API with the compiler.
    pub fn build(self) -> MimeInterpreter {
        MimeInterpreter {
            show_headers: self.show_headers,
            mime_body_interpreter: self.mime_body_interpreter,
        }
    }
}

/// MIME → MML message interpreter.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MimeInterpreter {
    show_headers: FilterHeaders,
    mime_body_interpreter: MimeBodyInterpreter,
}

impl MimeInterpreter {
    /// Interpret the given MIME [Message] as a MML [String].
    pub async fn from_msg(self, msg: &Message<'_>) -> Result<String> {
        let mut mml = String::new();

        match self.show_headers {
            FilterHeaders::All => msg.headers().iter().for_each(|header| {
                let key = header.name.as_str();
                let val = header::display_value(key, &header.value);
                mml.push_str(&format!("{key}: {val}\n"));
            }),
            FilterHeaders::Include(keys) => keys
                .iter()
                .filter_map(|key| msg.header(key.as_str()).map(|val| (key, val)))
                .for_each(|(key, val)| {
                    let val = header::display_value(key, val);
                    mml.push_str(&format!("{key}: {val}\n"));
                }),
            FilterHeaders::Exclude(keys) => msg
                .headers()
                .iter()
                .filter(|header| !keys.contains(&header.name.as_str().to_owned()))
                .for_each(|header| {
                    let key = header.name.as_str();
                    let val = header::display_value(key, &header.value);
                    mml.push_str(&format!("{key}: {val}\n"));
                }),
        };

        if !mml.is_empty() {
            mml.push('\n');
        }

        let mime_body_interpreter = self.mime_body_interpreter;

        #[cfg(feature = "pgp")]
        let mime_body_interpreter = mime_body_interpreter
            .with_pgp_sender(header::extract_first_email(msg.from()))
            .with_pgp_recipient(header::extract_first_email(msg.to()));

        let mml_body = mime_body_interpreter.interpret_msg(msg).await?;

        mml.push_str(&mml_body);

        Ok(mml)
    }

    /// Interpret the given MIME message bytes as a MML [String].
    pub async fn from_bytes(self, bytes: impl AsRef<[u8]>) -> Result<String> {
        let msg = MessageParser::new()
            .parse(bytes.as_ref())
            .ok_or(Error::ParseRawEmailError)?;
        self.from_msg(&msg).await
    }

    /// Interpret the given MIME [MessageBuilder] as a MML [String].
    pub async fn from_msg_builder(self, builder: MessageBuilder<'_>) -> Result<String> {
        let bytes = builder.write_to_vec().map_err(Error::BuildEmailError)?;
        self.from_bytes(&bytes).await
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use mail_builder::MessageBuilder;

    use super::MimeInterpreterBuilder;

    fn msg_builder() -> MessageBuilder<'static> {
        MessageBuilder::new()
            .message_id("id@localhost")
            .in_reply_to("reply-id@localhost")
            .date(0_u64)
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
    }

    #[tokio::test]
    async fn all_headers() {
        let mml = MimeInterpreterBuilder::new()
            .with_show_all_headers()
            .build()
            .from_msg_builder(msg_builder())
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "Message-ID: <id@localhost>",
            "In-Reply-To: <reply-id@localhost>",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "MIME-Version: 1.0",
            "Content-Type: text/plain; charset=utf-8",
            "Content-Transfer-Encoding: 7bit",
            "",
            "Hello, world!",
        );

        assert_eq!(mml, expected_mml);
    }

    #[tokio::test]
    async fn only_headers() {
        let mml = MimeInterpreterBuilder::new()
            .with_show_only_headers(["From", "Subject"])
            .build()
            .from_msg_builder(msg_builder())
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "From: from@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
        );

        assert_eq!(mml, expected_mml);
    }

    #[tokio::test]
    async fn only_headers_duplicated() {
        let mml = MimeInterpreterBuilder::new()
            .with_show_only_headers(["From", "Subject", "From"])
            .build()
            .from_msg_builder(msg_builder())
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "From: from@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
        );

        assert_eq!(mml, expected_mml);
    }

    #[tokio::test]
    async fn no_headers() {
        let mml = MimeInterpreterBuilder::new()
            .with_hide_all_headers()
            .build()
            .from_msg_builder(msg_builder())
            .await
            .unwrap();

        let expected_mml = concat_line!("Hello, world!");

        assert_eq!(mml, expected_mml);
    }

    #[tokio::test]
    async fn mml_markup_escaped() {
        let msg_builder = MessageBuilder::new()
            .message_id("id@localhost")
            .in_reply_to("reply-id@localhost")
            .date(0_u64)
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("<#part>Should be escaped.<#/part>");

        let mml = MimeInterpreterBuilder::new()
            .with_show_only_headers(["From", "Subject"])
            .build()
            .from_msg_builder(msg_builder)
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "From: from@localhost",
            "Subject: subject",
            "",
            "<#!part>Should be escaped.<#!/part>",
        );

        assert_eq!(mml, expected_mml);
    }
}
