pub mod attachment;
pub mod tpl;

#[cfg(feature = "imap-backend")]
use imap::types::{Fetch, Fetches};
use mail_parser::MimeHeaders;
use ouroboros::self_referencing;
use pimalaya_email_tpl::{Tpl, TplInterpreter};
use std::{borrow::Cow, fmt::Debug, io, path::PathBuf, result};
use thiserror::Error;
use tree_magic;

use crate::{account, AccountConfig};

pub use self::attachment::Attachment;
pub use self::tpl::*;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse email")]
    GetMailEntryError(#[source] maildirpp::Error),

    #[error("cannot get parsed version of email: {0}")]
    GetParsedEmailError(String),
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
    DecryptEmailPartError(#[source] pimalaya_process::Error),
    #[error("cannot verify signed email part")]
    VerifyEmailPartError(#[source] pimalaya_process::Error),

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
    InterpretEmailAsTplError(#[source] pimalaya_email_tpl::tpl::interpreter::Error),
}

#[derive(Debug, Error)]
enum ParsedBuilderError {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
}

pub type Result<T> = result::Result<T, Error>;

enum RawMessage<'a> {
    Cow(Cow<'a, [u8]>),
    #[cfg(feature = "imap-backend")]
    Fetch(&'a Fetch<'a>),
}

#[self_referencing]
pub struct Message<'a> {
    raw: RawMessage<'a>,
    #[borrows(mut raw)]
    #[covariant]
    parsed: result::Result<mail_parser::Message<'this>, ParsedBuilderError>,
}

impl Message<'_> {
    fn parsed_builder<'a>(
        raw: &'a mut RawMessage,
    ) -> result::Result<mail_parser::Message<'a>, ParsedBuilderError> {
        match raw {
            RawMessage::Cow(bytes) => {
                mail_parser::Message::parse(bytes).ok_or(ParsedBuilderError::ParseRawEmailError)
            }
            #[cfg(feature = "imap-backend")]
            RawMessage::Fetch(fetch) => {
                mail_parser::Message::parse(fetch.body().unwrap_or_default())
                    .ok_or(ParsedBuilderError::ParseRawEmailError)
            }
        }
    }

    pub fn parsed(&self) -> Result<&mail_parser::Message> {
        self.borrow_parsed()
            .as_ref()
            .map_err(|err| Error::GetParsedEmailError(err.to_string()))
    }

    pub fn raw(&self) -> Result<&[u8]> {
        self.parsed().map(|parsed| parsed.raw_message())
    }

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
                    mime: tree_magic::from_u8(part.contents()),
                    body: part.contents().to_owned(),
                }
            })
            .collect())
    }

    pub fn new_tpl_builder(config: &AccountConfig) -> NewTplBuilder {
        NewTplBuilder::new(config)
    }

    pub fn to_read_tpl(
        &self,
        config: &AccountConfig,
        with_interpreter: impl Fn(TplInterpreter) -> TplInterpreter,
    ) -> Result<Tpl> {
        let interpreter = config
            .generate_tpl_interpreter()
            .show_only_headers(config.email_reading_headers());
        with_interpreter(interpreter)
            .interpret_msg(self.parsed()?)
            .map_err(Error::InterpretEmailAsTplError)
    }

    pub fn to_reply_tpl_builder<'a>(&'a self, config: &'a AccountConfig) -> ReplyTplBuilder {
        ReplyTplBuilder::new(self, config)
    }

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

#[cfg(feature = "imap-backend")]
impl<'a> From<&'a Fetch<'a>> for Message<'a> {
    fn from(fetch: &'a Fetch) -> Self {
        MessageBuilder {
            raw: RawMessage::Fetch(fetch),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a mut maildirpp::MailEntry> for Message<'a> {
    fn from(entry: &'a mut maildirpp::MailEntry) -> Self {
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
    #[cfg(feature = "imap-backend")]
    Fetches(Fetches),
    MailEntries(Vec<maildirpp::MailEntry>),
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
            #[cfg(feature = "imap-backend")]
            RawMessages::Fetches(fetches) => fetches.iter().map(Message::from).collect(),
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

#[cfg(feature = "imap-backend")]
impl TryFrom<Fetches> for Messages {
    type Error = Error;

    fn try_from(fetches: Fetches) -> Result<Self> {
        if fetches.is_empty() {
            Err(Error::ParseEmailFromEmptyEntriesError)
        } else {
            Ok(MessagesBuilder {
                raw: RawMessages::Fetches(fetches),
                emails_builder: Messages::emails_builder,
            }
            .build())
        }
    }
}

impl TryFrom<Vec<maildirpp::MailEntry>> for Messages {
    type Error = Error;

    fn try_from(entries: Vec<maildirpp::MailEntry>) -> Result<Self> {
        if entries.is_empty() {
            Err(Error::ParseEmailFromEmptyEntriesError)
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

    use crate::{AccountConfig, Message};

    #[test]
    fn new_tpl_builder() {
        let config = AccountConfig {
            display_name: Some("From".into()),
            email: "from@localhost".into(),
            ..AccountConfig::default()
        };

        let tpl = Message::new_tpl_builder(&config).build().unwrap();

        let expected_tpl = concat_line!(
            "From: From <from@localhost>",
            "To: ",
            "Subject: ",
            "",
            "",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn new_tpl_builder_with_signature() {
        let config = AccountConfig {
            email: "from@localhost".into(),
            signature: Some("Regards,".into()),
            ..AccountConfig::default()
        };

        let tpl = Message::new_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_read_tpl() {
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

        let tpl = email.to_read_tpl(&config, |i| i).unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_read_tpl_with_show_all_headers() {
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
            .to_read_tpl(&config, |i| i.show_all_headers())
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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_read_tpl_with_show_only_headers() {
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
                i.show_only_headers([
                    // existing headers
                    "Subject",
                    "To",
                    // nonexisting header
                    "Content-Disposition",
                ])
            })
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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_read_tpl_with_email_reading_headers() {
        let config = AccountConfig {
            email_reading_headers: Some(vec!["X-Custom".into()]),
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
                i.show_additional_headers([
                    "Subject", // existing headers
                    "Cc", "Bcc", // nonexisting headers
                ])
            })
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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_builder() {
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

        let tpl = email.to_reply_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_builder_from_mailing_list() {
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

        let tpl = email.to_reply_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_builder_when_from_is_sender() {
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

        let tpl = email.to_reply_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    // TODO: In-Reply-To not valid, waiting for https://github.com/stalwartlabs/mail-parser/issues/53.
    fn to_reply_tpl_builder_with_reply_to() {
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

        let tpl = email.to_reply_tpl_builder(&config).build().unwrap();

        let expected_tpl = concat_line!(
            "From: to@localhost",
            "To: from2@localhost",
            "In-Reply-To: <id@localhost>",
            "Subject: RE:subject",
            "",
            "",
            "",
            "> Hello!",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_builder_with_signature() {
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

        let tpl = email.to_reply_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_all_tpl_builder() {
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
            .reply_all(true)
            .build()
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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_all_tpl_builder_with_reply_to() {
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
            .reply_all(true)
            .build()
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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_forward_tpl_builder() {
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

        let tpl = email.to_forward_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_forward_tpl_builder_with_date_and_signature() {
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

        let tpl = email.to_forward_tpl_builder(&config).build().unwrap();

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

        assert_eq!(*tpl, expected_tpl);
    }
}
