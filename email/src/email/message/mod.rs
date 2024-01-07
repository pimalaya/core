//! Module dedicated to email messages.
//!
//! The message is the content of the email, which is composed of a
//! header and a body.
//!
//! The core concept of this module is the [Message] structure, which
//! is just wrapper around the [mail_parser::Message] struct.

#[cfg(feature = "message-add")]
pub mod add;
#[cfg(feature = "message-add")]
pub mod add_with_flags;
pub mod attachment;
pub mod config;
#[cfg(feature = "message-copy")]
pub mod copy;
#[cfg(feature = "message-delete")]
pub mod delete;
#[cfg(feature = "message-get")]
pub mod get;
#[cfg(feature = "message-move")]
pub mod move_;
#[cfg(feature = "message-peek")]
pub mod peek;
#[cfg(feature = "message-send")]
pub mod send;
pub mod template;

#[cfg(feature = "imap")]
use imap::types::{Fetch, Fetches};
#[cfg(feature = "imap")]
use log::debug;
use mail_parser::{MessageParser, MimeHeaders};
#[cfg(feature = "maildir")]
use maildirpp::MailEntry;
use mml::MimeInterpreterBuilder;
use ouroboros::self_referencing;
use std::{borrow::Cow, fmt::Debug, io, path::PathBuf};
use thiserror::Error;

use crate::{
    account::{self, config::AccountConfig},
    Result,
};

#[doc(inline)]
pub use self::{
    attachment::Attachment,
    template::{ForwardTplBuilder, NewTplBuilder, ReplyTplBuilder},
};

/// Errors related to email messages.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse email")]
    ParseEmailError,
    #[error("cannot parse email: raw email is empty")]
    ParseEmailEmptyRawError,
    #[error("cannot delete local draft at {1}")]
    DeleteLocalDraftError(#[source] io::Error, PathBuf),

    #[error("cannot parse email: empty entries")]
    ParseEmailFromEmptyEntriesError,

    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error("cannot decrypt encrypted email part")]
    DecryptEmailPartError(#[source] process::Error),
    #[error("cannot verify signed email part")]
    VerifyEmailPartError(#[source] process::Error),

    // TODO: sort me
    #[error("cannot get content type of multipart")]
    GetMultipartContentTypeError,
    #[error("cannot find encrypted part of multipart")]
    GetEncryptedPartMultipartError,
    #[error("cannot parse encrypted part of multipart")]
    WriteEncryptedPartBodyError(#[source] io::Error),
    #[error("cannot write encrypted part to temporary file")]
    DecryptPartError(#[source] account::config::Error),

    #[error("cannot interpret email as template")]
    InterpretEmailAsTplError(#[source] mml::Error),

    #[error("cannot parse email message")]
    ParseEmailMessageError,
}

/// The raw message wrapper.
enum RawMessage<'a> {
    Cow(Cow<'a, [u8]>),
    #[cfg(feature = "imap")]
    Fetch(&'a Fetch<'a>),
}

/// The message wrapper.
#[self_referencing]
pub struct Message<'a> {
    raw: RawMessage<'a>,
    #[borrows(mut raw)]
    #[covariant]
    parsed: Option<mail_parser::Message<'this>>,
}

impl Message<'_> {
    /// Builds an optional message from a raw message.
    fn parsed_builder<'a>(raw: &'a mut RawMessage) -> Option<mail_parser::Message<'a>> {
        match raw {
            RawMessage::Cow(ref bytes) => MessageParser::new().parse(bytes.as_ref()),
            #[cfg(feature = "imap")]
            RawMessage::Fetch(fetch) => {
                MessageParser::new().parse(fetch.body().unwrap_or_default())
            }
        }
    }

    /// Returns the parsed version of the message.
    pub fn parsed(&self) -> Result<&mail_parser::Message> {
        let msg = self
            .borrow_parsed()
            .as_ref()
            .ok_or(Error::ParseEmailMessageError)?;
        Ok(msg)
    }

    /// Returns the raw version of the message.
    pub fn raw(&self) -> Result<&[u8]> {
        self.parsed().map(|parsed| parsed.raw_message())
    }

    /// Returns the list of message attachment.
    pub fn attachments(&self) -> Result<Vec<Attachment>> {
        Ok(self
            .parsed()?
            .attachments()
            .map(|part| {
                Attachment {
                    filename: part.attachment_name().map(ToOwned::to_owned),
                    // better to guess the real mime type from the
                    // body instead of using the one given from the
                    // content type
                    mime: tree_magic_mini::from_u8(part.contents()).to_owned(),
                    body: part.contents().to_owned(),
                }
            })
            .collect())
    }

    /// Creates a new template builder from an account configuration.
    pub fn new_tpl_builder(config: &AccountConfig) -> NewTplBuilder {
        NewTplBuilder::new(config)
    }

    /// Turns the current message into a read.
    pub async fn to_read_tpl(
        &self,
        config: &AccountConfig,
        with_interpreter: impl Fn(MimeInterpreterBuilder) -> MimeInterpreterBuilder,
    ) -> Result<String> {
        let interpreter = config
            .generate_tpl_interpreter()
            .with_show_only_headers(config.get_message_read_headers());
        let tpl = with_interpreter(interpreter)
            .build()
            .from_msg(self.parsed()?)
            .await
            .map_err(Error::InterpretEmailAsTplError)?;
        Ok(tpl)
    }

    /// Turns the current message into a reply template builder.
    ///
    /// The fact to return a template builder makes it easier to
    /// customize the final template from the outside.
    pub fn to_reply_tpl_builder<'a>(&'a self, config: &'a AccountConfig) -> ReplyTplBuilder {
        ReplyTplBuilder::new(self, config)
    }

    /// Turns the current message into a forward template builder.
    ///
    /// The fact to return a template builder makes it easier to
    /// customize the final template from the outside.
    pub fn to_forward_tpl_builder<'a>(&'a self, config: &'a AccountConfig) -> ForwardTplBuilder {
        ForwardTplBuilder::new(self, config)
    }
}

impl<'a> From<Vec<u8>> for Message<'a> {
    fn from(bytes: Vec<u8>) -> Self {
        MessageBuilder {
            raw: RawMessage::Cow(Cow::Owned(bytes)),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a [u8]> for Message<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        MessageBuilder {
            raw: RawMessage::Cow(Cow::Borrowed(bytes)),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

#[cfg(feature = "imap")]
impl<'a> From<&'a Fetch<'a>> for Message<'a> {
    fn from(fetch: &'a Fetch) -> Self {
        MessageBuilder {
            raw: RawMessage::Fetch(fetch),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

#[cfg(feature = "maildir")]
impl<'a> From<&'a mut MailEntry> for Message<'a> {
    fn from(entry: &'a mut MailEntry) -> Self {
        MessageBuilder {
            raw: RawMessage::Cow(Cow::Owned(entry.body().unwrap_or_default())),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a str> for Message<'a> {
    fn from(str: &'a str) -> Self {
        str.as_bytes().into()
    }
}

enum RawMessages {
    Vec(Vec<Vec<u8>>),
    #[cfg(feature = "imap")]
    Fetches(Fetches),
    #[cfg(feature = "maildir")]
    MailEntries(Vec<MailEntry>),
}

#[self_referencing]
pub struct Messages {
    raw: RawMessages,
    #[borrows(mut raw)]
    #[covariant]
    emails: Vec<Message<'this>>,
}

impl Messages {
    fn emails_builder<'a>(raw: &'a mut RawMessages) -> Vec<Message> {
        match raw {
            RawMessages::Vec(vec) => vec.iter().map(Vec::as_slice).map(Message::from).collect(),
            #[cfg(feature = "imap")]
            RawMessages::Fetches(fetches) => fetches
                .iter()
                .filter_map(|fetch| match fetch.body() {
                    Some(_) => Some(fetch),
                    None => {
                        debug!("skipping imap fetch with an empty body");
                        debug!("skipping imap fetch with an empty body: {fetch:#?}");
                        None
                    }
                })
                .map(Message::from)
                .collect(),
            #[cfg(feature = "maildir")]
            RawMessages::MailEntries(entries) => entries.iter_mut().map(Message::from).collect(),
        }
    }

    pub fn first(&self) -> Option<&Message> {
        self.borrow_emails().iter().next()
    }

    pub fn to_vec(&self) -> Vec<&Message> {
        self.borrow_emails().iter().collect()
    }
}

impl From<Vec<Vec<u8>>> for Messages {
    fn from(bytes: Vec<Vec<u8>>) -> Self {
        MessagesBuilder {
            raw: RawMessages::Vec(bytes),
            emails_builder: Messages::emails_builder,
        }
        .build()
    }
}

#[cfg(feature = "imap")]
impl TryFrom<Fetches> for Messages {
    type Error = crate::Error;

    fn try_from(fetches: Fetches) -> Result<Self> {
        if fetches.is_empty() {
            Err(Error::ParseEmailFromEmptyEntriesError.into())
        } else {
            Ok(MessagesBuilder {
                raw: RawMessages::Fetches(fetches),
                emails_builder: Messages::emails_builder,
            }
            .build())
        }
    }
}

#[cfg(feature = "maildir")]
impl TryFrom<Vec<MailEntry>> for Messages {
    type Error = crate::Error;

    fn try_from(entries: Vec<MailEntry>) -> Result<Self> {
        if entries.is_empty() {
            Err(Error::ParseEmailFromEmptyEntriesError.into())
        } else {
            Ok(MessagesBuilder {
                raw: RawMessages::MailEntries(entries),
                emails_builder: Messages::emails_builder,
            }
            .build())
        }
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;

    use crate::{
        account::config::AccountConfig,
        message::{config::MessageConfig, get::config::MessageReadConfig, Message},
    };

    #[tokio::test]
    async fn new_tpl_builder() {
        let config = AccountConfig {
            display_name: Some("From".into()),
            email: "from@localhost".into(),
            ..AccountConfig::default()
        };

        let tpl = Message::new_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: From <from@localhost>",
            "To: ",
            "Subject: ",
            "",
            "",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn new_tpl_builder_with_signature() {
        let config = AccountConfig {
            email: "from@localhost".into(),
            signature: Some("Regards,".into()),
            ..AccountConfig::default()
        };

        let tpl = Message::new_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: from@localhost",
            "To: ",
            "Subject: ",
            "",
            "",
            "",
            "-- ",
            "Regards,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_read_tpl() {
        let config = AccountConfig::default();
        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_read_tpl(&config, |i| i).await.unwrap();

        let expected_tpl = concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_read_tpl_with_show_all_headers() {
        let config = AccountConfig::default();
        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email
            .to_read_tpl(&config, |i| i.with_show_all_headers())
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_read_tpl_with_show_only_headers() {
        let config = AccountConfig::default();
        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let tpl = email
            .to_read_tpl(&config, |i| {
                i.with_show_only_headers([
                    // existing headers
                    "Subject",
                    "To",
                    // nonexisting header
                    "Content-Disposition",
                ])
            })
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "Subject: subject",
            "To: to@localhost",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_read_tpl_with_email_reading_headers() {
        let config = AccountConfig {
            message: Some(MessageConfig {
                read: Some(MessageReadConfig {
                    headers: Some(vec!["X-Custom".into()]),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "X-Custom: custom",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email
            .to_read_tpl(&config, |i| {
                i.with_show_additional_headers([
                    "Subject", // existing headers
                    "Cc", "Bcc", // nonexisting headers
                ])
            })
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "X-Custom: custom",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            ""
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_tpl_builder() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello,",
            "World!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_reply_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello,",
            "> World!",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_tpl_builder_from_mailing_list() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "Sender: mlist@localhost",
            "From: from@localhost",
            "To: mlist@localhost",
            "Cc: from@localhost,cc@localhost,cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: Re: subject",
            "",
            "Hello from mailing list!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_reply_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: mlist@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello from mailing list!",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_tpl_builder_when_from_is_sender() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: to@localhost",
            "Reply-To: reply-to@localhost",
            "To: from@localhost, from2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: Re: subject",
            "",
            "Hello back!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_reply_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost, from2@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello back!",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_tpl_builder_with_reply_to() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "Message-ID: <id@localhost>",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Reply-To: from2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: RE:subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_reply_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from2@localhost",
            "In-Reply-To: <id@localhost>",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_tpl_builder_with_signature() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,\n".into()),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_reply_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "",
            "-- ",
            "Cordialement,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_all_tpl_builder() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: from@localhost, to@localhost, cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email
            .to_reply_tpl_builder(&config)
            .with_reply_all(true)
            .build()
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from@localhost",
            "Cc: to2@localhost, cc@localhost, cc2@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_reply_all_tpl_builder_with_reply_to() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "Message-ID: <id@localhost>",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Reply-To: from2@localhost",
            "Cc: from@localhost, from2@localhost, to@localhost, <cc@localhost>, <cc2@localhost>",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email
            .to_reply_tpl_builder(&config)
            .with_reply_all(true)
            .build()
            .await
            .unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from2@localhost",
            "In-Reply-To: <id@localhost>",
            "Cc: to2@localhost, cc@localhost, cc2@localhost",
            "Subject: Re: subject",
            "",
            "",
            "",
            "> Hello!",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_forward_tpl_builder() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_forward_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: ",
            "Subject: Fwd: subject",
            "",
            "",
            "",
            "-------- Forwarded Message --------",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_forward_tpl_builder_with_date_and_signature() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,".into()),
            ..AccountConfig::default()
        };

        let email = Message::from(concat_line!(
            "Content-Type: text/plain",
            "Date: Thu, 10 Nov 2022 14:26:33 +0000",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let tpl = email.to_forward_tpl_builder(&config).build().await.unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: ",
            "Subject: Fwd: subject",
            "",
            "",
            "",
            "-- ",
            "Cordialement,",
            "",
            "-------- Forwarded Message --------",
            "Date: Thu, 10 Nov 2022 14:26:33 +0000",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            "",
        );

        assert_eq!(tpl, expected_tpl);
    }
}
