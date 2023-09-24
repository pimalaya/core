//! # MML to MIME message compilation module
//!
//! Module dedicated to MML → MIME message compilation.

use mail_builder::{headers::text::Text, MessageBuilder};
use mail_parser::Message;
use std::io;
use thiserror::Error;

#[cfg(feature = "pgp")]
use crate::{message::header, pgp::Pgp};
use crate::{message::MmlBodyCompiler, Result};

/// Errors dedicated to MML → MIME message compilation.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse template")]
    ParseMessageError,
    #[error("cannot parse MML message: empty body")]
    ParseMmlEmptyBodyError,
    #[error("cannot parse MML message: empty body content")]
    ParseMmlEmptyBodyContentError,
    #[error("cannot compile MML message to vec")]
    CompileMmlMessageToVecError(#[source] io::Error),
    #[error("cannot compile MML message to string")]
    CompileMmlMessageToStringError(#[source] io::Error),
}

/// The MML to MIME message compiler builder.
///
/// The compiler follows the builder pattern, where the build function
/// is named `compile`.
#[derive(Clone, Debug, Default)]
pub struct MmlCompilerBuilder {
    /// The internal MML to MIME message body compiler.
    mml_body_compiler: MmlBodyCompiler,
}

impl MmlCompilerBuilder {
    /// Create a new MML to MIME message compiler builder with default
    /// options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder option to customize PGP.
    #[cfg(feature = "pgp")]
    pub fn with_pgp(mut self, pgp: impl Into<Pgp>) -> Self {
        self.mml_body_compiler = self.mml_body_compiler.with_pgp(pgp.into());
        self
    }

    /// Build the final [MmlCompiler] based on the defined options.
    pub fn build<'a>(self, mml_msg: &'a str) -> Result<MmlCompiler<'a>> {
        let mml_msg = Message::parse(mml_msg.as_bytes()).ok_or(Error::ParseMessageError)?;
        let mml_body_compiler = self.mml_body_compiler;

        #[cfg(feature = "pgp")]
        let mml_body_compiler = mml_body_compiler
            .with_pgp_recipients(header::extract_emails(mml_msg.to()))
            .with_pgp_sender(header::extract_first_email(mml_msg.from()));

        Ok(MmlCompiler {
            mml_msg,
            mml_body_compiler,
        })
    }
}

/// The MML to MIME message compilation result structure.
///
/// This structure allows users to choose the final form of the
/// desired MIME message: [MessageBuilder], [Vec], [String] etc.
#[derive(Clone, Debug, Default)]
pub struct MmlCompiler<'a> {
    mml_msg: Message<'a>,
    mml_body_compiler: MmlBodyCompiler,
}

impl MmlCompiler<'_> {
    /// Compile the inner MML message into a [CompileMmlResult].
    ///
    /// The fact to return a intermediate structure allows users to
    /// customize the final form of the desired MIME message.
    pub async fn compile(&self) -> Result<CompileMmlResult<'_>> {
        let mml_body = self
            .mml_msg
            .text_bodies()
            .next()
            .ok_or(Error::ParseMmlEmptyBodyError)?
            .text_contents()
            .ok_or(Error::ParseMmlEmptyBodyContentError)?;

        let mml_body_compiler = &self.mml_body_compiler;

        let mut mime_msg_builder = mml_body_compiler.compile(mml_body).await?;

        mime_msg_builder = mime_msg_builder.header("MIME-Version", Text::new("1.0"));

        for header in self.mml_msg.headers() {
            let key = header.name.as_str();
            let val = super::header::to_builder_val(header);
            mime_msg_builder = mime_msg_builder.header(key, val);
        }

        Ok(CompileMmlResult { mime_msg_builder })
    }
}

/// The MML to MIME message compilation result.
///
/// This structure allows users to choose the final form of the
/// desired MIME message: [MessageBuilder], [Vec], [String] etc.
#[derive(Clone, Debug, Default)]
pub struct CompileMmlResult<'a> {
    mime_msg_builder: MessageBuilder<'a>,
}

impl<'a> CompileMmlResult<'a> {
    pub fn as_msg_builder(&self) -> &MessageBuilder {
        &self.mime_msg_builder
    }

    pub fn to_msg_builder(&self) -> MessageBuilder {
        self.mime_msg_builder.clone()
    }

    pub fn into_msg_builder(self) -> MessageBuilder<'a> {
        self.mime_msg_builder
    }

    pub fn into_vec(self) -> Result<Vec<u8>> {
        Ok(self
            .mime_msg_builder
            .write_to_vec()
            .map_err(Error::CompileMmlMessageToVecError)?)
    }

    pub fn into_string(self) -> Result<String> {
        Ok(self
            .mime_msg_builder
            .write_to_string()
            .map_err(Error::CompileMmlMessageToStringError)?)
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;

    use crate::{MimeInterpreterBuilder, MmlCompilerBuilder};

    #[tokio::test]
    async fn non_ascii_headers() {
        let mml = concat_line!(
            "Message-ID: <id@localhost>",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000",
            "From: Frȯm <from@localhost>",
            "To: Tó <to@localhost>",
            "Subject: Subjêct",
            "",
            "Hello, world!",
            "",
        );

        let mml_compiler = MmlCompilerBuilder::new().build(mml).unwrap();
        let mime_msg_builder = mml_compiler.compile().await.unwrap().into_msg_builder();

        let mml_msg = MimeInterpreterBuilder::new()
            .with_show_only_headers(["From", "To", "Subject"])
            .build()
            .from_msg_builder(mime_msg_builder)
            .await
            .unwrap();

        let expected_mml_msg = concat_line!(
            "From: Frȯm <from@localhost>",
            "To: Tó <to@localhost>",
            "Subject: Subjêct",
            "",
            "Hello, world!",
            "",
        );

        assert_eq!(mml_msg, expected_mml_msg);
    }

    #[tokio::test]
    async fn message_id_with_angles() {
        let mml = concat_line!(
            "From: Hugo Osvaldo Barrera <hugo@localhost>",
            "To: Hugo Osvaldo Barrera <hugo@localhost>",
            "Cc:",
            "Subject: Blah",
            "Message-ID: <bfb64e12-b7d4-474c-a658-8a221365f8ca@localhost>",
            "",
            "Test message",
            "",
        );

        let mml_compiler = MmlCompilerBuilder::new().build(mml).unwrap();
        let mime_msg_builder = mml_compiler.compile().await.unwrap().into_msg_builder();

        let mml_msg = MimeInterpreterBuilder::new()
            .with_show_only_headers(["Message-ID"])
            .build()
            .from_msg_builder(mime_msg_builder)
            .await
            .unwrap();

        let expected_mml_msg = concat_line!(
            "Message-ID: <bfb64e12-b7d4-474c-a658-8a221365f8ca@localhost>",
            "",
            "Test message",
            "",
        );

        assert_eq!(mml_msg, expected_mml_msg);
    }

    #[tokio::test]
    async fn message_id_without_angles() {
        let mml = concat_line!(
            "From: Hugo Osvaldo Barrera <hugo@localhost>",
            "To: Hugo Osvaldo Barrera <hugo@localhost>",
            "Cc:",
            "Subject: Blah",
            "Message-ID: bfb64e12-b7d4-474c-a658-8a221365f8ca@localhost",
            "",
            "Test message",
            "",
        );

        let mml_compiler = MmlCompilerBuilder::new().build(mml).unwrap();
        let mime_msg_builder = mml_compiler.compile().await.unwrap().into_msg_builder();

        let mml_msg = MimeInterpreterBuilder::new()
            .with_show_only_headers(["Message-ID"])
            .build()
            .from_msg_builder(mime_msg_builder)
            .await
            .unwrap();

        let expected_mml_msg = concat_line!(
            "Message-ID: <bfb64e12-b7d4-474c-a658-8a221365f8ca@localhost>",
            "",
            "Test message",
            "",
        );

        assert_eq!(mml_msg, expected_mml_msg);
    }

    #[tokio::test]
    async fn mml_markup_unescaped() {
        let mml = concat_line!(
            "Message-ID: <id@localhost>",
            "Date: Thu, 1 Jan 1970 00:00:00 +0000",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "<#!part>This should be unescaped<#!/part>",
            "",
        );

        let mml_compiler = MmlCompilerBuilder::new().build(mml).unwrap();
        let compile_mml_res = mml_compiler.compile().await.unwrap();
        let mime_msg_builder = compile_mml_res.clone().into_msg_builder();
        let mime_msg_str = compile_mml_res.into_string().unwrap();

        let mml_msg = MimeInterpreterBuilder::new()
            .with_show_only_headers(["From", "To", "Subject"])
            .build()
            .from_msg_builder(mime_msg_builder)
            .await
            .unwrap();

        let expected_mml_msg = concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "<#!part>This should be unescaped<#!/part>",
            "",
        );

        assert!(!mime_msg_str.contains("<#!part>"));
        assert!(mime_msg_str.contains("<#part>"));

        assert!(!mime_msg_str.contains("<#!/part>"));
        assert!(mime_msg_str.contains("<#/part>"));

        assert_eq!(mml_msg, expected_mml_msg);
    }
}
