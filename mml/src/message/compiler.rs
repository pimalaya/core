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
    #[error("cannot build message from template")]
    CreateMessageBuilderError,
    #[error("cannot compile template")]
    WriteTplToStringError(#[source] io::Error),
    #[error("cannot compile template")]
    WriteTplToVecError(#[source] io::Error),
    // #[error("cannot compile mime meta language")]
    // CompileMmlError(#[source] mml::compiler::Error),
    // #[error("cannot interpret email as a template")]
    // InterpretError(#[source] mml::interpreter::Error),
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

/// The MML to MIME message compiler.
///
/// The compiler follows the builder pattern, where the build function
/// is named `compile`.
#[derive(Clone, Debug, Default)]
pub struct MmlCompiler {
    /// The internal MML to MIME message body compiler.
    mml_body_compiler: MmlBodyCompiler,
}

impl MmlCompiler {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(feature = "pgp")]
    pub fn with_pgp(mut self, pgp: impl Into<Pgp>) -> Self {
        self.mml_body_compiler = self.mml_body_compiler.with_pgp(pgp.into());
        self
    }

    /// Compiles the given raw MIME message into a [MessageBuilder].
    ///
    /// The fact to return a message builder allows users to customize
    /// the final MIME message by adding custom headers, to adjust
    /// parts etc.
    pub fn compile<'a>(self, mml_msg: &'a str) -> Result<CompileMmlResult<'a>> {
        Ok(CompileMmlResult {
            mml_body_compiler: self.mml_body_compiler,
            fake_mime_msg: Message::parse(mml_msg.as_bytes()).ok_or(Error::ParseMessageError)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct CompileMmlResult<'a> {
    mml_body_compiler: MmlBodyCompiler,
    fake_mime_msg: Message<'a>,
}

impl<'a> CompileMmlResult<'a> {
    pub async fn to_msg_builder(&'a self) -> Result<MessageBuilder<'a>> {
        let mml_body = self
            .fake_mime_msg
            .text_bodies()
            .next()
            .ok_or(Error::ParseMmlEmptyBodyError)?
            .text_contents()
            .ok_or(Error::ParseMmlEmptyBodyContentError)?;

        let mml_body_compiler = &self.mml_body_compiler;

        #[cfg(feature = "pgp")]
        let mml_body_compiler = mml_body_compiler
            .with_pgp_recipients(header::extract_emails(mml_msg.to()))
            .with_pgp_sender(header::extract_first_email(mml_msg.from()));

        let mut mime_msg_builder = mml_body_compiler.compile(mml_body).await?;

        mime_msg_builder = mime_msg_builder.header("MIME-Version", Text::new("1.0"));

        for header in self.fake_mime_msg.headers() {
            let key = header.name.as_str();
            let val = super::header::to_builder_val(header);
            mime_msg_builder = mime_msg_builder.header(key, val);
        }

        Ok(mime_msg_builder)
    }

    pub async fn to_vec(&self) -> Result<Vec<u8>> {
        let msg_builder = self.to_msg_builder().await?;
        Ok(msg_builder
            .write_to_vec()
            .map_err(Error::CompileMmlMessageToVecError)?)
    }

    pub async fn to_string(&self) -> Result<String> {
        Ok(self
            .to_msg_builder()
            .await?
            .write_to_string()
            .map_err(Error::CompileMmlMessageToStringError)?)
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;

    use crate::{MimeInterpreter, MmlCompiler};

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

        let mml_compile_res = MmlCompiler::new().compile(mml).unwrap();
        let mime_msg_builder = mml_compile_res.to_msg_builder().await.unwrap();

        let mml_msg = MimeInterpreter::new()
            .with_show_only_headers(["From", "To", "Subject"])
            .interpret_msg_builder(mime_msg_builder)
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

        let mml_compile_res = MmlCompiler::new().compile(mml).unwrap();
        let mime_msg_builder = mml_compile_res.to_msg_builder().await.unwrap();

        let mml_msg = MimeInterpreter::new()
            .with_show_only_headers(["Message-ID"])
            .interpret_msg_builder(mime_msg_builder)
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

        let mml_compile_res = MmlCompiler::new().compile(mml).unwrap();
        let mime_msg_builder = mml_compile_res.to_msg_builder().await.unwrap();

        let mml_msg = MimeInterpreter::new()
            .with_show_only_headers(["Message-ID"])
            .interpret_msg_builder(mime_msg_builder)
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

        let mml_compile_res = MmlCompiler::new().compile(mml).unwrap();
        let mime_msg_builder = mml_compile_res.to_msg_builder().await.unwrap();
        let mime_msg_str = mml_compile_res.to_string().await.unwrap();

        let mml_msg = MimeInterpreter::new()
            .with_show_only_headers(["From", "To", "Subject"])
            .interpret_msg_builder(mime_msg_builder)
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
