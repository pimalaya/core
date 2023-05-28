#[cfg(feature = "imap-backend")]
use imap::types::{Fetch, Fetches};
use lettre::address::AddressError;
use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mail_parser::{Addr, HeaderValue, Message, MimeHeaders};
use maildir::{MailEntry, MailEntryError};
use mailparse::{MailParseError, ParsedMail};
use ouroboros::self_referencing;
use pimalaya_email_tpl::{Tpl, TplInterpreter};
use std::{borrow::Cow, fmt::Debug, io, path::PathBuf, result};
use thiserror::Error;
use tree_magic;

use crate::{account, AccountConfig, Attachment};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot parse email")]
    GetMailEntryError(#[source] MailEntryError),

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
    #[error("cannot parse email")]
    MailEntryError(#[source] MailEntryError),
    #[error("cannot parse raw email")]
    ParseRawEmailError,
}

pub type Result<T> = result::Result<T, Error>;

enum RawEmail<'a> {
    Vec(Vec<u8>),
    Slice(&'a [u8]),
    #[cfg(feature = "imap-backend")]
    Fetch(&'a Fetch<'a>),
    MailEntry(&'a mut MailEntry),
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
            RawEmail::MailEntry(entry) => Message::parse(
                entry
                    .parsed()
                    .map_err(ParsedBuilderError::MailEntryError)?
                    .raw_bytes,
            )
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

    /// Preconfigures a template builder for building new emails. It
    /// contains a "From" filled with the user's email address, an
    /// empty "To" and "Subject" and a text/plain part containing the
    /// user's signature (if existing). This function is useful when
    /// you need to compose a new email from scratch.
    pub fn new_tpl(
        config: &AccountConfig,
        additional_headers: Option<Vec<(String, String)>>,
        additional_body: Option<String>,
        interpreter: TplInterpreter,
    ) -> Result<Tpl> {
        let mut builder = MessageBuilder::new()
            .from(config.address())
            .to(Vec::<Address>::new())
            .subject("")
            .text_body({
                let mut lines = String::new();

                if let Some(ref body) = additional_body {
                    lines.push_str(body);
                    lines.push('\n');
                }

                if let Some(ref signature) = config.signature()? {
                    lines.push_str("\n\n");
                    lines.push_str(signature);
                }

                lines
            });

        // Additional headers

        if let Some(additional_headers) = additional_headers {
            for (key, val) in additional_headers {
                builder = builder.header(key, Raw::new(val));
            }
        }

        let tpl = interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }

    pub fn get_tpl_interpreter(config: &AccountConfig) -> TplInterpreter {
        TplInterpreter::new()
            .some_pgp_decrypt_cmd(config.email_reading_decrypt_cmd.clone())
            .some_pgp_verify_cmd(config.email_reading_verify_cmd.clone())
            .show_headers(["In-Reply-To", "From", "To", "Reply-To", "Cc", "Subject"])
            .show_some_headers(config.email_reading_headers.as_ref())
    }

    pub fn to_read_tpl(&self, interpreter: TplInterpreter) -> Result<Tpl> {
        let tpl = interpreter
            .interpret_msg(self.parsed()?)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }

    fn is_address_empty(header: &HeaderValue) -> bool {
        match header {
            HeaderValue::AddressList(addresses) => addresses.is_empty(),
            HeaderValue::Group(group) => group.addresses.is_empty(),
            HeaderValue::GroupList(groups) => groups.is_empty() || groups[0].addresses.is_empty(),
            HeaderValue::Empty => true,
            _ => false,
        }
    }

    fn contains_address(header: &HeaderValue, a: &Option<Cow<str>>) -> bool {
        match header {
            HeaderValue::Address(b) => a == &b.address,
            HeaderValue::AddressList(addresses) => {
                addresses.iter().find(|b| a == &b.address).is_some()
            }
            HeaderValue::Group(group) => group.addresses.iter().find(|b| a == &b.address).is_some(),
            HeaderValue::GroupList(groups) => groups
                .iter()
                .find(|group| group.addresses.iter().find(|b| a == &b.address).is_some())
                .is_some(),
            _ => false,
        }
    }

    fn get_address_id(header: &HeaderValue) -> Vec<String> {
        match header {
            HeaderValue::Address(a) => {
                vec![a.address.clone().unwrap_or_default().to_string()]
            }
            HeaderValue::AddressList(addresses) => addresses
                .iter()
                .map(|a| a.address.clone().unwrap_or_default().to_string())
                .collect(),
            HeaderValue::Group(group) => vec![group.name.clone().unwrap_or_default().to_string()],
            HeaderValue::GroupList(groups) => groups
                .iter()
                .map(|group| group.name.clone().unwrap_or_default().to_string())
                .collect(),
            _ => Vec::new(),
        }
    }

    fn into_address(header: HeaderValue) -> Address {
        match header {
            HeaderValue::Address(a) if a.address.is_some() => {
                Address::new_address(a.name, a.address.unwrap())
            }
            HeaderValue::AddressList(a) => Address::new_list(
                a.into_iter()
                    .filter_map(|a| a.address.map(|email| Address::new_address(a.name, email)))
                    .collect(),
            ),
            HeaderValue::Group(g) => Address::new_group(
                g.name,
                g.addresses
                    .into_iter()
                    .filter_map(|a| a.address.map(|email| Address::new_address(a.name, email)))
                    .collect(),
            ),
            _ => Address::new_list(Vec::new()),
        }
    }

    fn are_addresses_equal(a: &HeaderValue, b: &HeaderValue) -> bool {
        Self::get_address_id(a) == Self::get_address_id(b)
    }

    pub fn to_reply_tpl(
        &self,
        config: &AccountConfig,
        additional_headers: Option<Vec<(String, String)>>,
        additional_body: Option<String>,
        interpreter: TplInterpreter,
        all: bool,
    ) -> Result<Tpl> {
        let parsed = self.parsed()?;
        let mut builder = MessageBuilder::new();

        let me = Addr::new(Some(&config.name), &config.email);

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

        builder = builder.from(config.address());

        // To

        let recipients = if Self::are_addresses_equal(&sender, &to) {
            // when replying to an email received by a mailing list
            if Self::is_address_empty(&reply_to) {
                to.clone()
            } else {
                reply_to.clone()
            }
        } else if Self::are_addresses_equal(&from, &HeaderValue::Address(me.clone())) {
            // when replying to one of your own email
            to.clone()
        } else if Self::is_address_empty(&reply_to) {
            from.clone()
        } else {
            reply_to.clone()
        };

        builder = builder.to(Self::into_address(recipients.clone()));

        // Cc

        if all {
            builder = builder.cc({
                let cc = parsed.header("Cc").unwrap_or(&HeaderValue::Empty);
                let mut addresses = Vec::new();

                match to {
                    HeaderValue::Address(a) => {
                        if a.address != me.address
                            && !Self::contains_address(&from, &a.address)
                            && !Self::contains_address(&recipients, &a.address)
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
                                && !Self::contains_address(&from, &a.address)
                                && !Self::contains_address(&recipients, &a.address)
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
                            && !Self::contains_address(&from, &a.address)
                            && !Self::contains_address(&recipients, &a.address)
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
                                && !Self::contains_address(&from, &a.address)
                                && !Self::contains_address(&recipients, &a.address)
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

        if let Some(additional_headers) = additional_headers {
            for (key, val) in additional_headers {
                builder = builder.header(key, Raw::new(val));
            }
        }

        // Body

        builder = builder.text_body({
            let mut lines = String::new();

            if let Some(ref body) = additional_body {
                lines.push_str(body);
                lines.push('\n');
            }

            let body = TplInterpreter::new()
                .hide_all_headers()
                .hide_multipart_markup()
                .hide_part_markup()
                .sanitize_text_parts()
                .remove_text_plain_parts_signature()
                .interpret_msg(&parsed)
                .map_err(Error::InterpretEmailAsTplError)?;

            for line in body.lines() {
                lines.push('>');
                if !line.starts_with('>') {
                    lines.push(' ')
                }
                lines.push_str(line.trim_end_matches('\r'));
                lines.push('\n');
            }

            if let Some(ref signature) = config.signature()? {
                lines.push('\n');
                lines.push_str(signature);
            }

            lines
        });

        let tpl = interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }

    pub fn to_forward_tpl(
        &self,
        config: &AccountConfig,
        additional_headers: Option<Vec<(String, String)>>,
        additional_body: Option<String>,
        interpreter: TplInterpreter,
    ) -> Result<Tpl> {
        let parsed = self.parsed()?;
        let mut builder = MessageBuilder::new();

        // From

        builder = builder.from(config.address());

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

        if let Some(additional_headers) = additional_headers {
            for (key, val) in additional_headers {
                builder = builder.header(key, Raw::new(val));
            }
        }

        // Body

        builder = builder.text_body({
            let mut lines = String::from("\n");

            if let Some(ref body) = additional_body {
                lines.push_str(body);
                lines.push('\n');
            }

            if let Some(ref signature) = config.signature()? {
                lines.push('\n');
                lines.push_str(signature);
            }

            lines.push_str("\n\n-------- Forwarded Message --------\n");

            lines.push_str(
                &TplInterpreter::new()
                    .show_headers(["Date", "From", "To", "Cc", "Subject"])
                    .sanitize_text_parts()
                    .interpret_msg(&parsed)
                    .map_err(Error::InterpretEmailAsTplError)?,
            );

            lines
        });

        let tpl = interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
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

impl<'a> From<&'a mut MailEntry> for Email<'a> {
    fn from(entry: &'a mut MailEntry) -> Self {
        EmailBuilder {
            raw: RawEmail::MailEntry(entry),
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

enum RawEmails {
    Vec(Vec<Vec<u8>>),
    #[cfg(feature = "imap-backend")]
    Fetches(Fetches),
    MailEntries(Vec<MailEntry>),
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

impl TryFrom<Vec<MailEntry>> for Emails {
    type Error = Error;

    fn try_from(entries: Vec<MailEntry>) -> Result<Self> {
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
    fn new_tpl() {
        let config = AccountConfig {
            display_name: Some("From".into()),
            email: "from@localhost".into(),
            ..AccountConfig::default()
        };

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = Email::new_tpl(&config, None, None, interpreter).unwrap();

        let expected_tpl = concat_line!(
            "From: \"From\" <from@localhost>",
            "To: ",
            "Subject: ",
            "",
            "",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn new_tpl_with_signature() {
        let config = AccountConfig {
            email: "from@localhost".into(),
            signature: Some("Regards,".into()),
            ..AccountConfig::default()
        };

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = Email::new_tpl(&config, None, None, interpreter).unwrap();

        let expected_tpl = concat_line!(
            "From: <from@localhost>",
            "To: ",
            "Subject: ",
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
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
            "",
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email.to_read_tpl(interpreter).unwrap();

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
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config).show_all_headers();
        let tpl = email.to_read_tpl(interpreter).unwrap();

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
    fn to_read_tpl_with_show_only_headers() {
        let config = AccountConfig::default();
        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config)
            .hide_all_headers()
            .show_headers([
                // existing headers
                "Subject",
                "To",
                // nonexisting header
                "Content-Type",
            ]);
        let tpl = email.to_read_tpl(interpreter).unwrap();

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
            email_reading_headers: Some(vec!["From".into()]),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,",
        ));

        let interpreter = Email::get_tpl_interpreter(&config)
            .hide_all_headers()
            .show_headers([
                "Subject", // existing headers
                "Cc", "Bcc", // nonexisting headers
            ]);
        let tpl = email.to_read_tpl(interpreter).unwrap();

        let expected_tpl =
            concat_line!("Subject: subject", "", "Hello!", "", "-- ", "Regards,", "");

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, false)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: <from@localhost>",
            "Subject: Re: subject",
            "",
            "> Hello!",
            "",
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_from_mailing_list() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "Sender: mlist@localhost",
            "From: from@localhost",
            "To: mlist@localhost",
            "Cc: from@localhost, cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: Re: subject",
            "",
            "Hello from mailing list!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, false)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: <mlist@localhost>",
            "Subject: Re: subject",
            "",
            "> Hello from mailing list!",
            ""
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_when_from_is_sender() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
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
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, false)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: <from@localhost>, <from2@localhost>",
            "Subject: Re: subject",
            "",
            "> Hello back!",
            ""
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_with_reply_to() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
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
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, false)
            .unwrap();

        let expected_tpl = concat_line!(
            "In-Reply-To: <id@localhost>",
            "From: <to@localhost>",
            "To: <from2@localhost>",
            "Subject: RE:subject",
            "",
            "> Hello!",
            ""
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_tpl_with_signature() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,".into()),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, false)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: <from@localhost>",
            "Subject: Re: subject",
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
    fn to_reply_all_tpl() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: from@localhost, to@localhost, cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, true)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: <from@localhost>",
            "Cc: <to2@localhost>, <cc@localhost>, \r\n\t<cc2@localhost>",
            "Subject: Re: subject",
            "",
            "> Hello!",
            ""
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_reply_all_tpl_with_reply_to() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "Message-ID: <id@localhost>",
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Reply-To: from2@localhost",
            "Cc: from@localhost, from2@localhost, <to@localhost>, <cc@localhost>, <cc2@localhost>",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_reply_tpl(&config, None, None, interpreter, true)
            .unwrap();

        let expected_tpl = concat_line!(
            "In-Reply-To: <id@localhost>",
            "From: <to@localhost>",
            "To: <from2@localhost>",
            "Cc: <to2@localhost>, <cc@localhost>, \r\n\t<cc2@localhost>",
            "Subject: Re: subject",
            "",
            "> Hello!",
            ""
        );

        assert_eq!(*tpl, expected_tpl);
    }

    #[test]
    fn to_forward_tpl() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
            "From: from@localhost",
            "To: to@localhost, to2@localhost",
            "Cc: cc@localhost, cc2@localhost",
            "Bcc: bcc@localhost",
            "Subject: subject",
            "",
            "Hello!",
            "",
            "-- ",
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_forward_tpl(&config, None, None, interpreter)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: ",
            "Subject: Fwd: subject",
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
    fn to_forward_tpl_with_date_and_signature() {
        let config = AccountConfig {
            email: "to@localhost".into(),
            signature: Some("Cordialement,".into()),
            ..AccountConfig::default()
        };

        let email = Email::from(concat_line!(
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
            "Regards,"
        ));

        let interpreter = Email::get_tpl_interpreter(&config);
        let tpl = email
            .to_forward_tpl(&config, None, None, interpreter)
            .unwrap();

        let expected_tpl = concat_line!(
            "From: <to@localhost>",
            "To: ",
            "Subject: Fwd: subject",
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
