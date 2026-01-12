//! # MML to MIME message compilation module
//!
//! Module dedicated to MML → MIME message compilation.

use mail_builder::{headers::text::Text, MessageBuilder};
use mail_parser::{Message, MessageParser};

#[cfg(feature = "pgp")]
use crate::{message::header, pgp::Pgp};
use crate::{message::MmlBodyCompiler, Error, Result};

/// MML → MIME message compiler builder.
///
/// The compiler follows the builder pattern, where the build function
/// is named `compile`.
#[derive(Clone, Debug, Default)]
pub struct MmlCompilerBuilder {
    /// The internal MML to MIME message body compiler.
    mml_body_compiler: MmlBodyCompiler,
}

impl MmlCompilerBuilder {
    /// Create a new compiler builder with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Customize PGP.
    #[cfg(feature = "pgp")]
    pub fn set_pgp(&mut self, pgp: impl Into<Pgp>) {
        self.mml_body_compiler.set_pgp(pgp);
    }

    /// Customize PGP.
    #[cfg(feature = "pgp")]
    pub fn with_pgp(mut self, pgp: impl Into<Pgp>) -> Self {
        self.mml_body_compiler.set_pgp(pgp);
        self
    }

    /// Customize some PGP.
    #[cfg(feature = "pgp")]
    pub fn set_some_pgp(&mut self, pgp: Option<impl Into<Pgp>>) {
        self.mml_body_compiler.set_some_pgp(pgp);
    }

    /// Customize some PGP.
    #[cfg(feature = "pgp")]
    pub fn with_some_pgp(mut self, pgp: Option<impl Into<Pgp>>) -> Self {
        self.mml_body_compiler.set_some_pgp(pgp);
        self
    }

    /// Build the final [MmlCompiler] based on the defined options.
    pub fn build(self, mml_msg: &str) -> Result<MmlCompiler<'_>> {
        let mml_msg = MessageParser::new()
            .parse(mml_msg.as_bytes())
            .ok_or(Error::ParseMessageError)?;
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

/// MML → MIME message compiler.
///
/// This structure allows users to choose the final form of the
/// desired MIME message: [MessageBuilder], [Vec], [String] etc.
#[derive(Clone, Debug, Default)]
pub struct MmlCompiler<'a> {
    mml_msg: Message<'a>,
    mml_body_compiler: MmlBodyCompiler,
}

impl MmlCompiler<'_> {
    /// Compile the inner MML message into a [MmlCompileResult].
    ///
    /// The fact to return a intermediate structure allows users to
    /// customize the final form of the desired MIME message.
    pub async fn compile(&self) -> Result<MmlCompileResult<'_>> {
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

        Ok(MmlCompileResult { mime_msg_builder })
    }
}

/// MML → MIME message compilation result.
///
/// This structure allows users to choose the final form of the
/// desired MIME message: [MessageBuilder], [Vec], [String] etc.
#[derive(Clone, Debug, Default)]
pub struct MmlCompileResult<'a> {
    mime_msg_builder: MessageBuilder<'a>,
}

impl<'a> MmlCompileResult<'a> {
    /// Return a reference to the final MIME message builder.
    pub fn as_msg_builder(&self) -> &MessageBuilder<'_> {
        &self.mime_msg_builder
    }

    /// Return a copy of the final MIME message builder.
    pub fn to_msg_builder(&self) -> MessageBuilder<'_> {
        self.mime_msg_builder.clone()
    }

    /// Return the final MIME message builder.
    pub fn into_msg_builder(self) -> MessageBuilder<'a> {
        self.mime_msg_builder
    }

    /// Return the final MIME message as a [Vec].
    pub fn into_vec(self) -> Result<Vec<u8>> {
        self.mime_msg_builder
            .write_to_vec()
            .map_err(Error::CompileMmlMessageToVecError)
    }

    /// Return the final MIME message as a [String].
    pub fn into_string(self) -> Result<String> {
        self.mime_msg_builder
            .write_to_string()
            .map_err(Error::CompileMmlMessageToStringError)
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
