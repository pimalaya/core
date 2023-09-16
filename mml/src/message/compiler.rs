use mail_builder::{headers::text::Text, MessageBuilder};
use mail_parser::Message;
use std::io;
use thiserror::Error;

#[cfg(feature = "pgp")]
use crate::{message::header, Pgp};
use crate::{MmlBodyCompiler, Result};

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
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MmlCompiler {
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

    pub async fn compile<'a>(self, mime_msg: impl AsRef<[u8]>) -> Result<MessageBuilder<'a>> {
        let mime_msg = Message::parse(mime_msg.as_ref()).ok_or(Error::ParseMessageError)?;

        let mml_body = mime_msg
            .text_bodies()
            .into_iter()
            .filter_map(|part| part.text_contents())
            .fold(String::new(), |mut contents, content| {
                if !contents.is_empty() {
                    contents.push_str("\n\n");
                }
                contents.push_str(content.trim());
                contents
            });

        let mml_body_compiler = self.mml_body_compiler;

        #[cfg(feature = "pgp")]
        let mml_body_compiler = mml_body_compiler
            .with_pgp_recipients(header::extract_emails(mime_msg.to()))
            .with_pgp_sender(header::extract_first_email(mime_msg.from()));

        let mut mime_msg_builder = mml_body_compiler.compile(&mml_body).await?;

        mime_msg_builder = mime_msg_builder.header("MIME-Version", Text::new("1.0"));

        for header in mime_msg.headers() {
            let key = header.name.as_str().to_owned();
            let val = super::header::to_builder_val(header);
            mime_msg_builder = mime_msg_builder.header(key, val);
        }

        Ok(mime_msg_builder)
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

        let mime_msg = MmlCompiler::new().compile(mml).await.unwrap();

        let mml = MimeInterpreter::new()
            .with_show_only_headers(["From", "To", "Subject"])
            .interpret_msg_builder(mime_msg)
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "From: Frȯm <from@localhost>",
            "To: Tó <to@localhost>",
            "Subject: Subjêct",
            "",
            "Hello, world!",
            "",
        );

        assert_eq!(mml, expected_mml);
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

        let mime_msg = MmlCompiler::new().compile(mml).await.unwrap();

        let mml = MimeInterpreter::new()
            .with_show_only_headers(["Message-ID"])
            .interpret_msg_builder(mime_msg)
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "Message-ID: <bfb64e12-b7d4-474c-a658-8a221365f8ca@localhost>",
            "",
            "Test message",
            "",
        );

        assert_eq!(mml, expected_mml);
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

        let mime_msg = MmlCompiler::new().compile(mml).await.unwrap();

        let mml = MimeInterpreter::new()
            .with_show_only_headers(["Message-ID"])
            .interpret_msg_builder(mime_msg)
            .await
            .unwrap();

        let expected_mml = concat_line!(
            "Message-ID: <bfb64e12-b7d4-474c-a658-8a221365f8ca@localhost>",
            "",
            "Test message",
            "",
        );

        assert_eq!(mml, expected_mml);
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

        let mime_msg = MmlCompiler::new().compile(mml).await.unwrap();
        let mime_msg_str = mime_msg.clone().write_to_string().unwrap();

        let mml = MimeInterpreter::new()
            .with_show_only_headers(["From", "To", "Subject"])
            .interpret_msg_builder(mime_msg)
            .await
            .unwrap();

        let expected_mml = concat_line!(
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

        assert_eq!(mml, expected_mml);
    }
}
