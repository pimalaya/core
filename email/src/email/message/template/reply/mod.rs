//! # Reply template
//!
//! The main structure of this module is the [`ReplyTemplateBuilder`],
//! which helps you to build template in order to reply to a message.

pub mod config;

use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mail_parser::{Addr, HeaderValue};
use mml::MimeInterpreterBuilder;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

use crate::{
    account::config::AccountConfig,
    email::{address, error::Error},
    message::Message,
    Result,
};

use self::config::{ReplyTemplatePostingStyle, ReplyTemplateSignatureStyle};

use super::{Template, TemplateBody, TemplateCursor};

/// Regex used to trim out prefix(es) from a subject.
///
/// Everything starting by "Re:" (case and whitespace insensitive) is
/// considered a prefix.
static SUBJECT: Lazy<Regex> = Lazy::new(|| Regex::new("(?i:\\s*re\\s*:\\s*)*(.*)").unwrap());

/// Regex used to detect if an email address is a noreply one.
///
/// Matches usual names like `no_reply`, `noreply`, but also
/// `do-not.reply`.
static NO_REPLY: Lazy<Regex> = Lazy::new(|| Regex::new("(?i:not?[_\\-\\.]?reply)").unwrap());

/// Trim out prefix(es) from the given subject.
fn trim_prefix(subject: &str) -> &str {
    match SUBJECT.captures(subject).and_then(|cap| cap.get(1)) {
        Some(subject) => subject.as_str(),
        None => subject,
    }
}

/// The message reply template builder.
///
/// This builder helps you to create a template in order to reply to
/// an existing message.
pub struct ReplyTemplateBuilder<'a> {
    /// Reference to the current account configuration.
    config: Arc<AccountConfig>,

    /// Reference to the original message.
    msg: &'a Message<'a>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Should reply to all.
    reply_all: bool,

    /// Override the reply posting style.
    ///
    /// Uses the posting style from the account configuration if this
    /// one is `None`.
    posting_style: Option<ReplyTemplatePostingStyle>,

    /// Override the signature style.
    ///
    /// Uses the signature style from the account configuration if
    /// this one is `None`.
    signature_style: Option<ReplyTemplateSignatureStyle>,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,

    /// Template interpreter instance dedicated to the message thread.
    pub thread_interpreter: MimeInterpreterBuilder,
}

impl<'a> ReplyTemplateBuilder<'a> {
    /// Creates a reply template builder from an account configuration
    /// and a message references.
    pub fn new(msg: &'a Message, config: Arc<AccountConfig>) -> Self {
        let interpreter = config
            .generate_tpl_interpreter()
            .with_show_only_headers(config.get_message_write_headers());

        let thread_interpreter = config
            .generate_tpl_interpreter()
            .with_hide_all_headers()
            .with_show_plain_texts_signature(false)
            .with_show_attachments(false);

        Self {
            config,
            msg,
            headers: Vec::new(),
            body: String::new(),
            reply_all: false,
            posting_style: None,
            signature_style: None,
            interpreter,
            thread_interpreter,
        }
    }

    /// Sets additional template headers following the builder
    /// pattern.
    pub fn with_headers(
        mut self,
        headers: impl IntoIterator<Item = (impl ToString, impl ToString)>,
    ) -> Self {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    /// Sets some additional template headers following the builder
    /// pattern.
    pub fn with_some_headers(
        mut self,
        headers: Option<impl IntoIterator<Item = (impl ToString, impl ToString)>>,
    ) -> Self {
        if let Some(headers) = headers {
            self = self.with_headers(headers);
        }
        self
    }

    /// Sets the template body following the builder pattern.
    pub fn with_body(mut self, body: impl ToString) -> Self {
        self.body = body.to_string();
        self
    }

    /// Sets some template body following the builder pattern.
    pub fn with_some_body(mut self, body: Option<impl ToString>) -> Self {
        if let Some(body) = body {
            self = self.with_body(body)
        }
        self
    }

    /// Set some posting style.
    pub fn set_some_posting_style(&mut self, style: Option<impl Into<ReplyTemplatePostingStyle>>) {
        self.posting_style = style.map(Into::into);
    }

    /// Set the posting style.
    pub fn set_posting_style(&mut self, style: impl Into<ReplyTemplatePostingStyle>) {
        self.set_some_posting_style(Some(style));
    }

    /// Set some posting style, using the builder pattern.
    pub fn with_some_posting_style(
        mut self,
        style: Option<impl Into<ReplyTemplatePostingStyle>>,
    ) -> Self {
        self.set_some_posting_style(style);
        self
    }

    /// Set the posting style, using the builder pattern.
    pub fn with_posting_style(mut self, style: impl Into<ReplyTemplatePostingStyle>) -> Self {
        self.set_posting_style(style);
        self
    }

    /// Set some signature style.
    pub fn set_some_signature_style(
        &mut self,
        style: Option<impl Into<ReplyTemplateSignatureStyle>>,
    ) {
        self.signature_style = style.map(Into::into);
    }

    /// Set the signature style.
    pub fn set_signature_style(&mut self, style: impl Into<ReplyTemplateSignatureStyle>) {
        self.set_some_signature_style(Some(style));
    }

    /// Set some signature style, using the builder pattern.
    pub fn with_some_signature_style(
        mut self,
        style: Option<impl Into<ReplyTemplateSignatureStyle>>,
    ) -> Self {
        self.set_some_signature_style(style);
        self
    }

    /// Set the signature style, using the builder pattern.
    pub fn with_signature_style(mut self, style: impl Into<ReplyTemplateSignatureStyle>) -> Self {
        self.set_signature_style(style);
        self
    }

    /// Set the template interpreter following the builder pattern.
    pub fn with_interpreter(mut self, interpreter: MimeInterpreterBuilder) -> Self {
        self.interpreter = interpreter;
        self
    }

    /// Set the template thread interpreter following the builder
    /// pattern.
    pub fn with_thread_interpreter(mut self, interpreter: MimeInterpreterBuilder) -> Self {
        self.thread_interpreter = interpreter;
        self
    }

    /// Set the reply all flag following the builder pattern.
    pub fn with_reply_all(mut self, all: bool) -> Self {
        self.reply_all = all;
        self
    }

    /// Build the final reply message template.
    pub async fn build(self) -> Result<Template> {
        let mut cursor = TemplateCursor::default();

        let parsed = self.msg.parsed()?;
        let mut builder = MessageBuilder::new();

        let me = Addr::new(Some(&self.config.name), &self.config.email);

        let sender = parsed.header("Sender").unwrap_or(&HeaderValue::Empty);
        let from = parsed.header("From").unwrap_or(&HeaderValue::Empty);
        let to = parsed.header("To").unwrap_or(&HeaderValue::Empty);
        let reply_to = parsed.header("Reply-To").unwrap_or(&HeaderValue::Empty);

        let sig = self.config.find_full_signature();
        let sig_style = self
            .signature_style
            .unwrap_or_else(|| self.config.get_reply_template_signature_style());
        let posting_style = self
            .posting_style
            .unwrap_or_else(|| self.config.get_reply_template_posting_style());
        let quote_headline = self.config.get_reply_template_quote_headline(parsed);

        // In-Reply-To

        match parsed.header("Message-ID") {
            Some(HeaderValue::Text(message_id)) => {
                builder = builder.in_reply_to(vec![message_id.clone()]);
                cursor.row += 1;
            }
            Some(HeaderValue::TextList(message_id)) => {
                builder = builder.in_reply_to(message_id.clone());
                cursor.row += 1;
            }
            _ => (),
        }

        // From

        builder = builder.from(self.config.as_ref());
        cursor.row += 1;

        // To

        let i_am_the_sender = {
            let me = &HeaderValue::Address(mail_parser::Address::List(vec![me.clone()]));
            address::equal(from, me)
        };

        let i_am_a_main_recipient = address::contains(to, &me.address);

        let recipients = if i_am_the_sender {
            to.clone()
        } else if !i_am_a_main_recipient {
            if !address::is_empty(reply_to) {
                reply_to.clone()
            } else {
                to.clone()
            }
        } else if !address::is_empty(reply_to) {
            reply_to.clone()
        } else if !address::is_empty(from) {
            from.clone()
        } else {
            sender.clone()
        };

        builder = builder.to(address::into(recipients.clone()));
        cursor.row += 1;

        // Cc

        let cc = {
            let mut addresses = Vec::new();

            if !i_am_a_main_recipient && address::is_empty(reply_to) {
                if !address::is_empty(from) {
                    if let HeaderValue::Address(mail_parser::Address::List(addrs)) = &from {
                        for a in addrs {
                            if a.address == me.address {
                                continue;
                            }

                            if address::contains(&recipients, &a.address) {
                                continue;
                            }

                            if let Some(addr) = &a.address {
                                if NO_REPLY.is_match(addr) {
                                    continue;
                                }
                            }

                            addresses.push(Address::new_address(
                                a.name.clone(),
                                a.address.clone().unwrap(),
                            ));
                        }
                    }
                } else if let HeaderValue::Address(mail_parser::Address::List(addrs)) = &sender {
                    for a in addrs {
                        if a.address == me.address {
                            continue;
                        }

                        if address::contains(&recipients, &a.address) {
                            continue;
                        }

                        if let Some(addr) = &a.address {
                            if NO_REPLY.is_match(addr) {
                                continue;
                            }
                        }

                        addresses.push(Address::new_address(
                            a.name.clone(),
                            a.address.clone().unwrap(),
                        ));
                    }
                }
            }

            if self.reply_all {
                let cc = parsed.header("Cc").unwrap_or(&HeaderValue::Empty);

                if let HeaderValue::Address(mail_parser::Address::List(addrs)) = cc {
                    for a in addrs {
                        if a.address == me.address {
                            continue;
                        }

                        if address::contains(&reply_to, &a.address) {
                            continue;
                        }

                        if address::contains(&from, &a.address) {
                            continue;
                        }

                        if address::contains(&sender, &a.address) {
                            continue;
                        }

                        if address::contains(&recipients, &a.address) {
                            continue;
                        }

                        if let Some(addr) = &a.address {
                            if NO_REPLY.is_match(addr) {
                                continue;
                            }
                        }

                        addresses.push(Address::new_address(
                            a.name.clone(),
                            a.address.clone().unwrap(),
                        ));
                    }
                }
            }

            if addresses.is_empty() {
                None
            } else {
                Some(Address::new_list(addresses))
            }
        };

        if let Some(cc) = cc {
            builder = builder.cc(cc);
            cursor.row += 1;
        }

        // Subject

        // TODO: make this customizable?
        let prefix = String::from("Re: ");
        let subject = trim_prefix(parsed.subject().unwrap_or_default());

        builder = builder.subject(prefix + subject);
        cursor.row += 1;

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
            cursor.row += 1;
        }

        // Body

        builder = builder.text_body({
            let mut body = TemplateBody::new(cursor);

            let reply_body = self
                .thread_interpreter
                .build()
                .from_msg(parsed)
                .await
                .map_err(Error::InterpretMessageAsThreadTemplateError)?;
            let reply_body = reply_body.trim();

            if !reply_body.is_empty() && posting_style.is_bottom() {
                if let Some(ref hline) = quote_headline {
                    body.push_str(hline);
                }

                for line in reply_body.lines() {
                    body.push('>');
                    if !line.starts_with('>') {
                        body.push(' ')
                    }
                    body.push_str(line);
                    body.push('\n');
                }

                // drop last line feed
                body.pop();
                body.flush();
            }

            // when interleaved posting style, only push non-empty
            // body and do not lock the cursor (it must be locked at
            // the beginning of the quote)
            if posting_style.is_interleaved() {
                if !self.body.is_empty() {
                    body.push_str(&self.body);
                    body.flush();
                }
            }
            // when bottom or top posting style, push the body and
            // lock the cursor at the end of it
            else {
                body.push_str(&self.body);
                body.flush();
                body.cursor.lock();
            }

            // NOTE: hide this block for interleaved posting style?
            if sig_style.is_above_quote() {
                if let Some(ref sig) = sig {
                    body.push_str(sig);
                    body.flush();
                }
            }

            if !reply_body.is_empty() && !posting_style.is_bottom() {
                if posting_style.is_top() {
                    if let Some(ref hline) = quote_headline {
                        body.push_str(hline);
                    }
                }

                let mut lines_count = 0;
                for line in reply_body.lines() {
                    lines_count += 1;

                    body.push('>');
                    if !line.starts_with('>') {
                        body.push(' ')
                    }
                    body.push_str(line);
                    body.push('\n');
                }

                // drop last line feed
                body.pop();
                body.flush();

                // if interleaved posting style, put the cursor at the
                // beginning of the quote instead of leaving it at the
                // end
                if posting_style.is_interleaved() {
                    body.cursor.row -= lines_count - 1;
                    body.cursor.col = 0;
                }
            }

            if sig_style.is_below_quote() {
                if let Some(ref sig) = sig {
                    body.push_str(sig);
                    body.flush();
                }
            }

            cursor = body.cursor.clone();
            body
        });

        if sig_style.is_attached() {
            if let Some(sig) = sig {
                builder = builder.attachment("text/plain", "signature.txt", sig)
            }
        }

        let content = self
            .interpreter
            .build()
            .from_msg_builder(builder)
            .await
            .map_err(Error::InterpretMessageAsTemplateError)?;

        Ok(Template::new_with_cursor(content, cursor))
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use std::sync::Arc;

    use crate::{
        account::config::AccountConfig,
        message::Message,
        template::{
            reply::{
                config::{ReplyTemplatePostingStyle, ReplyTemplateSignatureStyle},
                ReplyTemplateBuilder,
            },
            Template,
        },
    };

    #[tokio::test]
    async fn default() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "", // cursor here
                ),
                (5, 0),
            ),
        );
    }

    #[tokio::test]
    async fn with_body() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "", // cursor here
                    "",
                    "> Hello, world!",
                ),
                (5, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with single line body
                .with_body("Hello, back!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "Hello, back!", // cursor here
                    "",
                    "> Hello, world!",
                ),
                (5, 12),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with multi lines body
                .with_body("\n\nHello\n,\nworld!\n\n!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "",
                    "",
                    "Hello",
                    ",",
                    "world!",
                    "",
                    "!", // cursor here
                    "",
                    "> Hello, world!",
                ),
                (11, 1),
            ),
        );
    }

    #[tokio::test]
    async fn with_signature() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature: Some("signature".into()),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "", // cursor here
                    "",
                    "> Hello, world!",
                    "",
                    "-- ",
                    "signature",
                ),
                (5, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force signature above quote
                .with_signature_style(ReplyTemplateSignatureStyle::AboveQuote)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "", // cursor here
                    "",
                    "-- ",
                    "signature",
                    "",
                    "> Hello, world!",
                ),
                (5, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force signature hidden
                .with_signature_style(ReplyTemplateSignatureStyle::Hidden)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "", // cursor here
                    "",
                    "> Hello, world!",
                ),
                (5, 0),
            ),
        );
    }

    #[tokio::test]
    async fn with_quote() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force the bottom-posting style
                .with_posting_style(ReplyTemplatePostingStyle::Bottom)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "> Hello, world!",
                    "",
                    "", // cursor here
                ),
                (7, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force the interleaved posting style
                .with_posting_style(ReplyTemplatePostingStyle::Interleaved)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "> Hello, world!", // cursor here
                ),
                (5, 0),
            ),
        );
    }

    #[tokio::test]
    async fn with_body_and_signature() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature: Some("signature".into()),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "", // cursor here
                    "",
                    "> Hello, world!",
                    "",
                    "-- ",
                    "signature"
                ),
                (5, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with single line body
                .with_body("Hello, back!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "Hello, back!", // cursor here
                    "",
                    "> Hello, world!",
                    "",
                    "-- ",
                    "signature"
                ),
                (5, 12),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with single line body
                .with_body("Hello, back!")
                // force signature above quote
                .with_signature_style(ReplyTemplateSignatureStyle::AboveQuote)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "Hello, back!", // cursor here
                    "",
                    "-- ",
                    "signature",
                    "",
                    "> Hello, world!",
                ),
                (5, 12),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with multi lines body
                .with_body("\n\nHello\n,\nworld!\n\n!")
                // force signature hidden
                .with_signature_style(ReplyTemplateSignatureStyle::Hidden)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "",
                    "",
                    "Hello",
                    ",",
                    "world!",
                    "",
                    "!", // cursor here
                    "",
                    "> Hello, world!",
                ),
                (11, 1),
            ),
        );
    }

    #[tokio::test]
    async fn with_signature_and_quote() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature: Some("signature".into()),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force signature above quote
                .with_signature_style(ReplyTemplateSignatureStyle::AboveQuote)
                // force bottom-posting style
                .with_posting_style(ReplyTemplatePostingStyle::Bottom)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "> Hello, world!",
                    "",
                    "", // cursor here
                    "",
                    "-- ",
                    "signature",
                ),
                (7, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force signature hidden
                .with_signature_style(ReplyTemplateSignatureStyle::Hidden)
                // force bottom-posting style
                .with_posting_style(ReplyTemplatePostingStyle::Bottom)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "> Hello, world!",
                    "",
                    "", // cursor here
                ),
                (7, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force signature above quote
                .with_signature_style(ReplyTemplateSignatureStyle::AboveQuote)
                // force interleaved posting style
                .with_posting_style(ReplyTemplatePostingStyle::Interleaved)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "-- ",
                    "signature",
                    "",
                    "> Hello, world!", // cursor here
                ),
                (8, 0),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // force signature hidden
                .with_signature_style(ReplyTemplateSignatureStyle::Hidden)
                // force interleaved posting style
                .with_posting_style(ReplyTemplatePostingStyle::Interleaved)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "> Hello, world!", // cursor here
                ),
                (5, 0),
            ),
        );
    }

    #[tokio::test]
    async fn with_body_and_quote() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with single line body
                .with_body("Hello, back!")
                // force the bottom-posting style with body
                .with_posting_style(ReplyTemplatePostingStyle::Bottom)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "> Hello, world!",
                    "",
                    "Hello, back!", // cursor here
                ),
                (7, 12),
            ),
        );

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with single line body
                .with_body("Hello, back!")
                // force the interleaved posting style with body
                .with_posting_style(ReplyTemplatePostingStyle::Interleaved)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "Hello, back!",
                    "",
                    "> Hello, world!", // cursor here
                ),
                (7, 0),
            ),
        );
    }

    #[tokio::test]
    async fn with_body_signature_and_quote() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature: Some("signature".into()),
            ..Default::default()
        });

        let msg = &Message::from(concat_line!(
            "Content-Type: text/plain",
            "From: sender@localhost",
            "To: me@localhost",
            "Subject: subject",
            "",
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ReplyTemplateBuilder::new(msg, config.clone())
                // with single line body
                .with_body("Hello, back!")
                // force signature above quote
                .with_signature_style(ReplyTemplateSignatureStyle::AboveQuote)
                // force interleaved posting style
                .with_posting_style(ReplyTemplatePostingStyle::Interleaved)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: sender@localhost",
                    "Subject: Re: subject",
                    "",
                    "Hello, back!",
                    "",
                    "-- ",
                    "signature",
                    "",
                    "> Hello, world!", // cursor here
                ),
                (10, 0),
            ),
        );
    }

    #[test]
    fn trim_subject_prefix() {
        assert_eq!(super::trim_prefix("Hello, world!"), "Hello, world!");
        assert_eq!(super::trim_prefix("re:Hello, world!"), "Hello, world!");
        assert_eq!(super::trim_prefix("Re   :Hello, world!"), "Hello, world!");
        assert_eq!(super::trim_prefix("rE:   Hello, world!"), "Hello, world!");
        assert_eq!(
            super::trim_prefix("  RE:  re  :Hello, world!"),
            "Hello, world!"
        );
    }
}
