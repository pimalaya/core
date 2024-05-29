//! Module dedicated to email messages.
//!
//! The message is the content of the email, which is composed of a
//! header and a body.
//!
//! The core concept of this module is the [Message] structure, which
//! is just wrapper around the [mail_parser::Message] struct.

pub mod add;
pub mod attachment;
pub mod config;
pub mod copy;
pub mod delete;
pub mod get;
#[cfg(feature = "imap")]
pub mod imap;
pub mod r#move;
pub mod peek;
pub mod remove;
pub mod send;
#[cfg(feature = "account-sync")]
pub mod sync;
pub mod template;

use std::{borrow::Cow, sync::Arc};

use imap_client::types::{core::Vec1, fetch::MessageDataItem};
use mail_parser::{MessageParser, MimeHeaders};
use maildirpp::MailEntry;
use mml::MimeInterpreterBuilder;
use ouroboros::self_referencing;

use self::{
    attachment::Attachment,
    template::{
        forward::ForwardTemplateBuilder, new::NewTemplateBuilder, reply::ReplyTemplateBuilder,
    },
};
use crate::{account::config::AccountConfig, debug, email::error::Error, trace};

/// The message wrapper.
#[self_referencing]
pub struct Message<'a> {
    bytes: Cow<'a, [u8]>,
    #[borrows(mut bytes)]
    #[covariant]
    parsed: Option<mail_parser::Message<'this>>,
}

impl Message<'_> {
    /// Builds an optional message from a raw message.
    fn parsed_builder<'a>(bytes: &'a mut Cow<[u8]>) -> Option<mail_parser::Message<'a>> {
        MessageParser::new().parse((*bytes).as_ref())
    }

    /// Returns the parsed version of the message.
    pub fn parsed(&self) -> Result<&mail_parser::Message, Error> {
        let msg = self
            .borrow_parsed()
            .as_ref()
            .ok_or(Error::ParseEmailMessageError)?;
        Ok(msg)
    }

    /// Returns the raw version of the message.
    pub fn raw(&self) -> Result<&[u8], Error> {
        self.parsed().map(|parsed| parsed.raw_message())
    }

    /// Returns the list of message attachment.
    pub fn attachments(&self) -> Result<Vec<Attachment>, Error> {
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
    pub fn new_tpl_builder(config: Arc<AccountConfig>) -> NewTemplateBuilder {
        NewTemplateBuilder::new(config)
    }

    /// Turns the current message into a read.
    pub async fn to_read_tpl(
        &self,
        config: &AccountConfig,
        with_interpreter: impl Fn(MimeInterpreterBuilder) -> MimeInterpreterBuilder,
    ) -> Result<String, Error> {
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
    pub fn to_reply_tpl_builder(&self, config: Arc<AccountConfig>) -> ReplyTemplateBuilder {
        ReplyTemplateBuilder::new(self, config)
    }

    /// Turns the current message into a forward template builder.
    ///
    /// The fact to return a template builder makes it easier to
    /// customize the final template from the outside.
    pub fn to_forward_tpl_builder(&self, config: Arc<AccountConfig>) -> ForwardTemplateBuilder {
        ForwardTemplateBuilder::new(self, config)
    }
}

impl<'a> From<Vec<u8>> for Message<'a> {
    fn from(bytes: Vec<u8>) -> Self {
        MessageBuilder {
            bytes: Cow::Owned(bytes),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a [u8]> for Message<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        MessageBuilder {
            bytes: Cow::Borrowed(bytes),
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

// TODO: move to maildir module
impl<'a> From<&'a mut MailEntry> for Message<'a> {
    fn from(entry: &'a mut MailEntry) -> Self {
        MessageBuilder {
            bytes: Cow::Owned(entry.body().unwrap_or_default()),
            parsed_builder: Message::parsed_builder,
        }
        .build()
    }
}

enum RawMessages {
    #[cfg(feature = "imap")]
    Imap(Vec<Vec1<MessageDataItem<'static>>>),
    #[cfg(feature = "maildir")]
    MailEntries(Vec<MailEntry>),
    #[cfg(feature = "notmuch")]
    Notmuch(Vec<Vec<u8>>),
}

#[self_referencing]
pub struct Messages {
    raw: RawMessages,
    #[borrows(mut raw)]
    #[covariant]
    emails: Vec<Message<'this>>,
}

impl Messages {
    fn emails_builder<'a>(raw: &'a mut RawMessages) -> Vec<Message<'a>> {
        match raw {
            #[cfg(feature = "imap")]
            RawMessages::Imap(items) => items
                .iter()
                .filter_map(|items| match Message::try_from(items.as_ref()) {
                    Ok(msg) => Some(msg),
                    Err(err) => {
                        debug!("cannot build imap message: {err}");
                        trace!("{err:#?}");
                        None
                    }
                })
                .collect(),
            #[cfg(feature = "maildir")]
            RawMessages::MailEntries(entries) => entries.iter_mut().map(Message::from).collect(),
            #[cfg(feature = "notmuch")]
            RawMessages::Notmuch(raw) => raw
                .iter()
                .map(|raw| Message::from(raw.as_slice()))
                .collect(),
        }
    }

    pub fn first(&self) -> Option<&Message> {
        self.borrow_emails().iter().next()
    }

    pub fn to_vec(&self) -> Vec<&Message> {
        self.borrow_emails().iter().collect()
    }
}

#[cfg(feature = "imap")]
impl From<Vec<Vec1<MessageDataItem<'static>>>> for Messages {
    fn from(items: Vec<Vec1<MessageDataItem<'static>>>) -> Self {
        MessagesBuilder {
            raw: RawMessages::Imap(items),
            emails_builder: Messages::emails_builder,
        }
        .build()
    }
}

#[cfg(feature = "maildir")]
impl TryFrom<Vec<MailEntry>> for Messages {
    type Error = Error;

    fn try_from(entries: Vec<MailEntry>) -> Result<Self, Error> {
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

#[cfg(feature = "notmuch")]
impl From<Vec<Vec<u8>>> for Messages {
    fn from(raw: Vec<Vec<u8>>) -> Self {
        MessagesBuilder {
            raw: RawMessages::Notmuch(raw),
            emails_builder: Messages::emails_builder,
        }
        .build()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use concat_with::concat_line;

    use crate::{
        account::config::AccountConfig,
        message::{config::MessageConfig, get::config::MessageReadConfig, Message},
        template::Template,
    };

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
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_forward_tpl_builder() {
        let config = Arc::new(AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        });

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

        let tpl = email.to_forward_tpl_builder(config).build().await.unwrap();

        let expected_tpl = Template::new_with_cursor(
            concat_line!(
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
            ),
            (5, 0),
        );

        assert_eq!(tpl, expected_tpl);
    }

    #[tokio::test]
    async fn to_forward_tpl_builder_with_date_and_signature() {
        let config = Arc::new(AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,".into()),
            ..AccountConfig::default()
        });

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

        let tpl = email.to_forward_tpl_builder(config).build().await.unwrap();

        let expected_tpl = Template::new_with_cursor(
            concat_line!(
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
            ),
            (5, 0),
        );

        assert_eq!(tpl, expected_tpl);
    }
}
