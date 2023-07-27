use log::{debug, warn};
use mail_builder::MessageBuilder;
use mail_parser::Message;
use pimalaya_keyring::Entry;
use std::{io, path::PathBuf, result};
use thiserror::Error;

use crate::{mml, FilterParts, Tpl};

use super::header;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
    #[error("cannot build email")]
    BuildEmailError(#[source] io::Error),
    #[error("cannot interpret email body as mml")]
    InterpretMmlError(#[source] mml::interpreter::Error),
    #[error("cannot parse empty sender")]
    ParseSenderEmptyError,
    #[error("cannot parse empty recipient")]
    ParseRecipientEmptyError,

    #[error("cannot get pgp secret key from keyring")]
    GetSecretKeyFromKeyringError(pimalaya_keyring::Error),
    #[error("cannot read pgp secret key from keyring")]
    ReadSecretKeyFromKeyringError(pimalaya_pgp::Error),
    #[error("cannot read pgp secret key from path {1}")]
    ReadSecretKeyFromPathError(pimalaya_pgp::Error, PathBuf),
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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Decrypt {
    #[default]
    None,
    Path(String),
    Keyring(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Verify {
    #[default]
    None,
    KeyServers(Vec<String>),
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

    /// PGP decrypt configuration.
    pgp_decrypt: Decrypt,

    /// PGP verify configuration.
    pgp_verify: Verify,

    mml_interpreter: mml::Interpreter,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_show_headers(mut self, s: ShowHeadersStrategy) -> Self {
        self.show_headers = s;
        self
    }

    pub fn with_show_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::All;
        self
    }

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
        self.show_headers = ShowHeadersStrategy::Only(headers);
        self
    }

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
            ShowHeadersStrategy::All => {
                self.show_headers = ShowHeadersStrategy::Only(next_headers);
            }
            ShowHeadersStrategy::Only(headers) => {
                headers.extend(next_headers);
            }
        };

        self
    }

    pub fn with_hide_all_headers(mut self) -> Self {
        self.show_headers = ShowHeadersStrategy::Only(Vec::new());
        self
    }

    pub fn with_show_multiparts(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_multiparts(b);
        self
    }

    pub fn with_filter_parts(mut self, f: FilterParts) -> Self {
        self.mml_interpreter = self.mml_interpreter.filter_parts(f);
        self
    }

    pub fn with_show_plain_texts_signature(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_plain_texts_signature(b);
        self
    }

    pub fn with_show_attachments(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_attachments(b);
        self
    }

    pub fn with_show_inline_attachments(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.show_inline_attachments(b);
        self
    }

    pub fn with_save_attachments(mut self, b: bool) -> Self {
        self.mml_interpreter = self.mml_interpreter.save_attachments(b);
        self
    }

    pub fn with_save_attachments_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.mml_interpreter = self.mml_interpreter.save_attachments_dir(dir);
        self
    }

    pub fn with_pgp_decrypt(mut self, decrypt: Decrypt) -> Self {
        self.pgp_decrypt = decrypt;
        self
    }

    pub fn with_pgp_verify(mut self, verify: Verify) -> Self {
        self.pgp_verify = verify;
        self
    }

    /// Interprets the given [`mail_parser::Message`] as a
    /// [`crate::Tpl`].
    pub async fn interpret_msg(self, msg: &Message<'_>) -> Result<Tpl> {
        let mut tpl = Tpl::new();

        let sender = header::extract_first_email(msg.from()).ok_or(Error::ParseSenderEmptyError)?;
        let recipient =
            header::extract_first_email(msg.to()).ok_or(Error::ParseRecipientEmptyError)?;

        match self.show_headers {
            ShowHeadersStrategy::All => msg.headers().iter().for_each(|header| {
                let key = header.name.as_str();
                let val = header::display_value(key, &header.value);
                tpl.push_str(&format!("{key}: {val}\n"));
            }),
            ShowHeadersStrategy::Only(keys) => keys
                .iter()
                .filter_map(|key| msg.header(key).map(|val| (key, val)))
                .for_each(|(key, val)| {
                    let val = header::display_value(key, val);
                    tpl.push_str(&format!("{key}: {val}\n"));
                }),
        };

        if !tpl.is_empty() {
            tpl.push_str("\n");
        }

        let mml = self
            .mml_interpreter
            .clone()
            .with_pgp_decrypt_key(match &self.pgp_decrypt {
                Decrypt::None => {
                    warn!("cannot set pgp decrypt key: decrypt not configured");
                    None
                }
                Decrypt::Path(path_tpl) => {
                    let get_skey = || async {
                        let path = PathBuf::from(path_tpl.replace("<recipient>", &recipient));
                        let skey = pimalaya_pgp::read_signed_secret_key_from_path(path.clone())
                            .await
                            .map_err(|err| Error::ReadSecretKeyFromPathError(err, path))?;
                        Result::Ok(skey)
                    };
                    match get_skey().await {
                        Ok(skey) => Some(skey),
                        Err(err) => {
                            warn!("cannot get pgp secret key from path: {err}");
                            debug!("cannot get pgp secret key from path: {err:?}");
                            None
                        }
                    }
                }
                Decrypt::Keyring(entry_tpl) => {
                    let get_skey = || async {
                        let data = Entry::from(entry_tpl.replace("<recipient>", &recipient))
                            .get_secret()
                            .map_err(Error::GetSecretKeyFromKeyringError)?;
                        let skey = pimalaya_pgp::read_skey_from_string(data)
                            .await
                            .map_err(Error::ReadSecretKeyFromKeyringError)?;
                        Result::Ok(skey)
                    };
                    match get_skey().await {
                        Ok(skey) => Some(skey),
                        Err(err) => {
                            warn!("cannot get pgp secret key from keyring: {err}");
                            debug!("cannot get pgp secret key from keyring: {err:?}");
                            None
                        }
                    }
                }
            })
            .with_pgp_verify_key(match &self.pgp_verify {
                Verify::None => {
                    warn!("cannot set pgp verify key: verify not configured");
                    None
                }
                Verify::KeyServers(key_servers) => {
                    let wkd_pkey = pimalaya_pgp::wkd::get_one(recipient.clone()).await;

                    match wkd_pkey {
                        Ok(pkey) => Some(pkey),
                        Err(err) => {
                            warn!("cannot get public key of {sender} using wkd: {err}");
                            debug!("cannot get public key of {sender} using wkd: {err:?}");

                            let hkps_pkey =
                                pimalaya_pgp::hkps::get_one(recipient, key_servers.clone()).await;

                            match hkps_pkey {
                                Ok(pkey) => Some(pkey),
                                Err(err) => {
                                    warn!("cannot get public key of {sender} using hkps: {err}");
                                    debug!("cannot get public key of {sender} using hkps: {err:?}");
                                    None
                                }
                            }
                        }
                    }
                }
            })
            .interpret_msg(msg)
            .await
            .map_err(Error::InterpretMmlError)?;

        tpl.push_str(mml.trim_end());
        tpl.push('\n');

        Ok(tpl)
    }

    /// Interprets the given bytes as a [`crate::Tpl`].
    pub async fn interpret_bytes<B: AsRef<[u8]>>(self, bytes: B) -> Result<Tpl> {
        let msg = Message::parse(bytes.as_ref()).ok_or(Error::ParseRawEmailError)?;
        self.interpret_msg(&msg).await
    }

    /// Interprets the given [`mail_builder::MessageBuilder`] as a
    /// [`crate::Tpl`].
    pub async fn interpret_msg_builder(self, builder: MessageBuilder<'_>) -> Result<Tpl> {
        let bytes = builder.write_to_vec().map_err(Error::BuildEmailError)?;
        self.interpret_bytes(&bytes).await
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
            .in_reply_to("reply-id@localhost")
            .date(0 as u64)
            .from("from@localhost")
            .to("to@localhost")
            .subject("subject")
            .text_body("Hello, world!")
    }

    #[tokio::test]
    async fn all_headers() {
        let tpl = Interpreter::new()
            .with_show_all_headers()
            .interpret_msg_builder(msg())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "Message-ID: <id@localhost>",
            "In-Reply-To: <reply-id@localhost>",
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

    #[tokio::test]
    async fn only_headers() {
        let tpl = Interpreter::new()
            .with_show_only_headers(["From", "Subject"])
            .interpret_msg_builder(msg())
            .await
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

    #[tokio::test]
    async fn only_headers_duplicated() {
        let tpl = Interpreter::new()
            .with_show_only_headers(["From", "Subject", "From"])
            .interpret_msg_builder(msg())
            .await
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

    #[tokio::test]
    async fn no_headers() {
        let tpl = Interpreter::new()
            .with_hide_all_headers()
            .interpret_msg_builder(msg())
            .await
            .unwrap();

        let expected_tpl = concat_line!("Hello, world!", "");

        assert_eq!(*tpl, expected_tpl);
    }
}
