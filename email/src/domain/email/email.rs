#[cfg(feature = "imap-backend")]
use imap::types::{Fetch, Fetches};
use lettre::address::AddressError;
use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mail_parser::{Addr, HeaderValue, Message, MimeHeaders};
use mailparse::{MailParseError, ParsedMail};
use ouroboros::self_referencing;
use pimalaya_email_tpl::{Tpl, TplInterpreter};
use std::{fmt::Debug, io, path::PathBuf, result};
use thiserror::Error;
use tree_magic;

use crate::{account, AccountConfig, Attachment};

use super::address;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse email")]
    GetMailEntryError(#[source] maildirpp::Error),

    #[error("cannot get parsed version of email: {0}")]
    GetParsedEmailError(String),
    #[error("cannot parse email")]
    ParseEmailError(#[source] MailParseError),
    #[error("cannot parse email body")]
    ParseEmailBodyError(#[source] MailParseError),
    #[error("cannot parse email: raw email is empty")]
    ParseEmailEmptyRawError,
    #[error("cannot parse message or address")]
    ParseEmailAddressError(#[from] AddressError),
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
    ParseEncryptedPartError(#[source] mailparse::MailParseError),
    #[error("cannot get body from encrypted part")]
    GetEncryptedPartBodyError(#[source] mailparse::MailParseError),
    #[error("cannot write encrypted part to temporary file")]
    WriteEncryptedPartBodyError(#[source] io::Error),
    #[error("cannot write encrypted part to temporary file")]
    DecryptPartError(#[source] account::config::Error),

    #[error("cannot interpret email as template")]
    InterpretEmailAsTplError(#[source] pimalaya_email_tpl::tpl::interpreter::Error),
    #[error("cannot parse raw message")]
    ParseRawMessageError,
    #[error("cannot build forward template")]
    BuildForwardTplError(#[source] io::Error),
}

#[derive(Debug, Error)]
enum ParsedBuilderError {
    #[error("cannot parse raw email")]
    ParseRawEmailError,
}

pub type Result<T> = result::Result<T, Error>;

enum RawEmail<'a> {
    Vec(Vec<u8>),
    Slice(&'a [u8]),
    #[cfg(feature = "imap-backend")]
    Fetch(&'a Fetch<'a>),
}

#[self_referencing]
pub struct Email<'a> {
    raw: RawEmail<'a>,
    #[borrows(mut raw)]
    #[covariant]
    parsed: result::Result<Message<'this>, ParsedBuilderError>,
}

impl Email<'_> {
    fn parsed_builder<'a>(
        raw: &'a mut RawEmail,
    ) -> result::Result<Message<'a>, ParsedBuilderError> {
        match raw {
            RawEmail::Vec(bytes) => {
                Message::parse(bytes).ok_or(ParsedBuilderError::ParseRawEmailError)
            }
            RawEmail::Slice(bytes) => {
                Message::parse(bytes).ok_or(ParsedBuilderError::ParseRawEmailError)
            }
            #[cfg(feature = "imap-backend")]
            RawEmail::Fetch(fetch) => Message::parse(fetch.body().unwrap_or_default())
                .ok_or(ParsedBuilderError::ParseRawEmailError),
        }
    }

    pub fn parsed(&self) -> Result<&Message> {
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

impl<'a> From<Vec<u8>> for Email<'a> {
    fn from(bytes: Vec<u8>) -> Self {
        EmailBuilder {
            raw: RawEmail::Vec(bytes),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a [u8]> for Email<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        EmailBuilder {
            raw: RawEmail::Slice(bytes),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<ParsedMail<'a>> for Email<'a> {
    fn from(parsed: ParsedMail<'a>) -> Self {
        EmailBuilder {
            raw: RawEmail::Slice(parsed.raw_bytes),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

#[cfg(feature = "imap-backend")]
impl<'a> From<&'a Fetch<'a>> for Email<'a> {
    fn from(fetch: &'a Fetch) -> Self {
        EmailBuilder {
            raw: RawEmail::Fetch(fetch),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a mut maildirpp::MailEntry> for Email<'a> {
    fn from(entry: &'a mut maildirpp::MailEntry) -> Self {
        EmailBuilder {
            raw: RawEmail::Vec(entry.body().unwrap_or_default()),
            parsed_builder: Email::parsed_builder,
        }
        .build()
    }
}

impl<'a> From<&'a str> for Email<'a> {
    fn from(str: &'a str) -> Self {
        str.as_bytes().into()
    }
}

pub struct NewTplBuilder<'a> {
    config: &'a AccountConfig,
    headers: Vec<(String, String)>,
    body: String,
    pub thread_interpreter: TplInterpreter,
    pub interpreter: TplInterpreter,
    reply_all: bool,
}

impl<'a> NewTplBuilder<'a> {
    pub fn new(config: &'a AccountConfig) -> Self {
        Self {
            config,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(config.email_writing_headers()),
            thread_interpreter: config
                .generate_tpl_interpreter()
                .hide_all_headers()
                .show_plain_texts_signature(false)
                .show_attachments(false),
            reply_all: false,
        }
    }

    pub fn headers<K, V>(mut self, headers: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    pub fn some_headers<K, V>(mut self, headers: Option<impl IntoIterator<Item = (K, V)>>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        if let Some(headers) = headers {
            self = self.headers(headers);
        }
        self
    }

    pub fn body(mut self, body: impl ToString) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn some_body(mut self, body: Option<impl ToString>) -> Self {
        if let Some(body) = body {
            self = self.body(body)
        }
        self
    }

    pub fn interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.interpreter = interpreter;
        self
    }

    pub fn thread_interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.thread_interpreter = interpreter;
        self
    }

    pub fn reply_all(mut self, all: bool) -> Self {
        self.reply_all = all;
        self
    }

    pub fn build(self) -> Result<Tpl> {
        let mut builder = MessageBuilder::new()
            .from(self.config.addr())
            .to(Vec::<Address>::new())
            .subject("")
            .text_body({
                let mut lines = String::new();

                if !self.body.is_empty() {
                    lines.push_str(&self.body);
                    lines.push('\n');
                }

                if let Some(ref signature) = self.config.signature()? {
                    lines.push_str("\n\n");
                    lines.push_str(signature);
                }

                lines
            });

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
        }

        let tpl = self
            .interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }
}

pub struct ReplyTplBuilder<'a> {
    email: &'a Email<'a>,
    config: &'a AccountConfig,
    headers: Vec<(String, String)>,
    body: String,
    pub thread_interpreter: TplInterpreter,
    pub interpreter: TplInterpreter,
    reply_all: bool,
}

impl<'a> ReplyTplBuilder<'a> {
    pub fn new(email: &'a Email, config: &'a AccountConfig) -> Self {
        Self {
            email,
            config,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(config.email_writing_headers()),
            thread_interpreter: config
                .generate_tpl_interpreter()
                .hide_all_headers()
                .show_plain_texts_signature(false)
                .show_attachments(false),
            reply_all: false,
        }
    }

    pub fn headers<K, V>(mut self, headers: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    pub fn some_headers<K, V>(mut self, headers: Option<impl IntoIterator<Item = (K, V)>>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        if let Some(headers) = headers {
            self = self.headers(headers);
        }
        self
    }

    pub fn body(mut self, body: impl ToString) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn some_body(mut self, body: Option<impl ToString>) -> Self {
        if let Some(body) = body {
            self = self.body(body)
        }
        self
    }

    pub fn interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.interpreter = interpreter;
        self
    }

    pub fn thread_interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.thread_interpreter = interpreter;
        self
    }

    pub fn reply_all(mut self, all: bool) -> Self {
        self.reply_all = all;
        self
    }

    pub fn build(self) -> Result<Tpl> {
        let parsed = self.email.parsed()?;
        let mut builder = MessageBuilder::new();

        let me = Addr::new(Some(&self.config.name), &self.config.email);

        let sender = parsed.header("Sender").unwrap_or(&HeaderValue::Empty);
        let from = parsed.header("From").unwrap_or(&HeaderValue::Empty);
        let to = parsed.header("To").unwrap_or(&HeaderValue::Empty);
        let reply_to = parsed.header("Reply-To").unwrap_or(&HeaderValue::Empty);

        // In-Reply-To

        match parsed.header("Message-ID") {
            Some(HeaderValue::Text(message_id)) => {
                builder = builder.in_reply_to(vec![message_id.clone()]);
            }
            Some(HeaderValue::TextList(message_id)) => {
                builder = builder.in_reply_to(message_id.clone());
            }
            _ => (),
        }

        // From

        builder = builder.from(self.config.addr());

        // To

        let recipients = if address::equal(&sender, &to) {
            // when replying to an email received by a mailing list
            if address::is_empty(&reply_to) {
                to.clone()
            } else {
                reply_to.clone()
            }
        } else if address::equal(&from, &HeaderValue::Address(me.clone())) {
            // when replying to one of your own email
            to.clone()
        } else if address::is_empty(&reply_to) {
            from.clone()
        } else {
            reply_to.clone()
        };

        builder = builder.to(address::into(recipients.clone()));

        // Cc

        if self.reply_all {
            builder = builder.cc({
                let cc = parsed.header("Cc").unwrap_or(&HeaderValue::Empty);
                let mut addresses = Vec::new();

                match to {
                    HeaderValue::Address(a) => {
                        if a.address != me.address
                            && !address::contains(&from, &a.address)
                            && !address::contains(&recipients, &a.address)
                        {
                            addresses.push(Address::new_address(
                                a.name.clone(),
                                a.address.clone().unwrap(),
                            ));
                        }
                    }
                    HeaderValue::AddressList(a) => {
                        for a in a {
                            if a.address != me.address
                                && !address::contains(&from, &a.address)
                                && !address::contains(&recipients, &a.address)
                            {
                                addresses.push(Address::new_address(
                                    a.name.clone(),
                                    a.address.clone().unwrap(),
                                ));
                            }
                        }
                    }
                    _ => (),
                }

                match cc {
                    HeaderValue::Address(a) => {
                        if a.address != me.address
                            && !address::contains(&from, &a.address)
                            && !address::contains(&recipients, &a.address)
                        {
                            addresses.push(Address::new_address(
                                a.name.clone(),
                                a.address.clone().unwrap(),
                            ));
                        }
                    }
                    HeaderValue::AddressList(a) => {
                        for a in a {
                            if a.address != me.address
                                && !address::contains(&from, &a.address)
                                && !address::contains(&recipients, &a.address)
                            {
                                addresses.push(Address::new_address(
                                    a.name.clone(),
                                    a.address.clone().unwrap(),
                                ));
                            }
                        }
                    }
                    _ => (),
                }

                Address::new_list(addresses)
            });
        }

        // Subject

        let subject = parsed
            .header("Subject")
            .cloned()
            .map(|h| h.unwrap_text())
            .unwrap_or_default();

        builder = builder.subject(if subject.to_lowercase().starts_with("re:") {
            subject
        } else {
            format!("Re: {subject}").into()
        });

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
        }

        // Body

        builder = builder.text_body({
            let mut lines = String::from("\n\n");

            if !self.body.is_empty() {
                lines.push_str(&self.body);
                lines.push('\n');
            }

            let body = self
                .thread_interpreter
                .interpret_msg(&parsed)
                .map_err(Error::InterpretEmailAsTplError)?;

            for line in body.trim().lines() {
                lines.push('>');
                if !line.starts_with('>') {
                    lines.push(' ')
                }
                lines.push_str(&line);
                lines.push('\n');
            }

            if let Some(ref signature) = self.config.signature()? {
                lines.push('\n');
                lines.push_str(signature);
            }

            lines.trim_end().to_owned()
        });

        let tpl = self
            .interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }
}

pub struct ForwardTplBuilder<'a> {
    email: &'a Email<'a>,
    config: &'a AccountConfig,
    headers: Vec<(String, String)>,
    body: String,
    pub interpreter: TplInterpreter,
    pub thread_interpreter: TplInterpreter,
}

impl<'a> ForwardTplBuilder<'a> {
    pub fn new(email: &'a Email, config: &'a AccountConfig) -> Self {
        Self {
            email,
            config,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(config.email_writing_headers()),
            thread_interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(["Date", "From", "To", "Cc", "Subject"])
                .save_attachments(true),
        }
    }

    pub fn headers<K, V>(mut self, headers: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    pub fn some_headers<K, V>(mut self, headers: Option<impl IntoIterator<Item = (K, V)>>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        if let Some(headers) = headers {
            self = self.headers(headers);
        }
        self
    }

    pub fn body(mut self, body: impl ToString) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn some_body(mut self, body: Option<impl ToString>) -> Self {
        if let Some(body) = body {
            self = self.body(body)
        }
        self
    }

    pub fn interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.interpreter = interpreter;
        self
    }

    pub fn thread_interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.thread_interpreter = interpreter;
        self
    }

    pub fn build(self) -> Result<Tpl> {
        let parsed = self.email.parsed()?;
        let mut builder = MessageBuilder::new();

        // From

        builder = builder.from(self.config.addr());

        // To

        builder = builder.to(Vec::<Address>::new());

        // Subject

        let subject = parsed
            .header("Subject")
            .cloned()
            .map(|h| h.unwrap_text())
            .unwrap_or_default();

        builder = builder.subject(if subject.to_lowercase().starts_with("fwd:") {
            subject
        } else {
            format!("Fwd: {subject}").into()
        });

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
        }

        // Body

        builder = builder.text_body({
            let mut lines = String::from("\n");

            if !self.body.is_empty() {
                lines.push('\n');
                lines.push_str(&self.body);
                lines.push('\n');
            }

            if let Some(ref signature) = self.config.signature()? {
                lines.push('\n');
                lines.push_str(signature);
                lines.push('\n');
            }

            lines.push_str("\n-------- Forwarded Message --------\n");

            lines.push_str(
                &self
                    .thread_interpreter
                    .interpret_msg(&parsed)
                    .map_err(Error::InterpretEmailAsTplError)?,
            );

            lines.trim_end().to_owned()
        });

        let tpl = self
            .interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }
}

enum RawEmails {
    Vec(Vec<Vec<u8>>),
    #[cfg(feature = "imap-backend")]
    Fetches(Fetches),
    MailEntries(Vec<maildirpp::MailEntry>),
}

#[self_referencing]
pub struct Emails {
    raw: RawEmails,
    #[borrows(mut raw)]
    #[covariant]
    emails: Vec<Email<'this>>,
}

impl Emails {
    fn emails_builder<'a>(raw: &'a mut RawEmails) -> Vec<Email> {
        match raw {
            RawEmails::Vec(vec) => vec.iter().map(Vec::as_slice).map(Email::from).collect(),
            #[cfg(feature = "imap-backend")]
            RawEmails::Fetches(fetches) => fetches.iter().map(Email::from).collect(),
            RawEmails::MailEntries(entries) => entries.iter_mut().map(Email::from).collect(),
        }
    }

    pub fn first(&self) -> Option<&Email> {
        self.borrow_emails().iter().next()
    }

    pub fn to_vec(&self) -> Vec<&Email> {
        self.borrow_emails().iter().collect()
    }
}

impl From<Vec<Vec<u8>>> for Emails {
    fn from(bytes: Vec<Vec<u8>>) -> Self {
        EmailsBuilder {
            raw: RawEmails::Vec(bytes),
            emails_builder: Emails::emails_builder,
        }
        .build()
    }
}

#[cfg(feature = "imap-backend")]
impl TryFrom<Fetches> for Emails {
    type Error = Error;

    fn try_from(fetches: Fetches) -> Result<Self> {
        if fetches.is_empty() {
            Err(Error::ParseEmailFromEmptyEntriesError)
        } else {
            Ok(EmailsBuilder {
                raw: RawEmails::Fetches(fetches),
                emails_builder: Emails::emails_builder,
            }
            .build())
        }
    }
}

impl TryFrom<Vec<maildirpp::MailEntry>> for Emails {
    type Error = Error;

    fn try_from(entries: Vec<maildirpp::MailEntry>) -> Result<Self> {
        if entries.is_empty() {
            Err(Error::ParseEmailFromEmptyEntriesError)
        } else {
            Ok(EmailsBuilder {
                raw: RawEmails::MailEntries(entries),
                emails_builder: Emails::emails_builder,
            }
            .build())
        }
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;

    use crate::{AccountConfig, Email};

    #[test]
    fn new_tpl_builder() {
        let config = AccountConfig {
            display_name: Some("From".into()),
            email: "from@localhost".into(),
            ..AccountConfig::default()
        };

        let tpl = Email::new_tpl_builder(&config).build().unwrap();

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

        let tpl = Email::new_tpl_builder(&config).build().unwrap();

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
        let email = Email::from(concat_line!(
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
        let email = Email::from(concat_line!(
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
        let email = Email::from(concat_line!(
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

        let email = Email::from(concat_line!(
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

        let email = Email::from(concat_line!(
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

        let email = Email::from(concat_line!(
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

        let email = Email::from(concat_line!(
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
            "To: from@localhost,from2@localhost",
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
    fn to_reply_tpl_builder_with_reply_to() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
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
            "In-Reply-To: id@localhost",
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

        let email = Email::from(concat_line!(
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

        let email = Email::from(concat_line!(
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
            "Cc: to2@localhost,cc@localhost,cc2@localhost",
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

        let email = Email::from(concat_line!(
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
            "In-Reply-To: id@localhost",
            "Cc: to2@localhost,cc@localhost,cc2@localhost",
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

        let email = Email::from(concat_line!(
            "Content-Type: text/plain",
            "From: from@localhost",
            "To: to@localhost,to2@localhost",
            "Cc: cc@localhost,cc2@localhost",
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
            "To: to@localhost,to2@localhost",
            "Cc: cc@localhost,cc2@localhost",
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

        let email = Email::from(concat_line!(
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
            "To: to@localhost,to2@localhost",
            "Cc: cc@localhost,cc2@localhost",
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
