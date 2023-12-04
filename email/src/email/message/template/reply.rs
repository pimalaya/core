//! Module dedicated to email message reply template.
//!
//! The main structure of this module is the [ReplyTplBuilder], which
//! helps you to build template in order to reply to a message.

use log::warn;
use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mail_parser::{Addr, HeaderValue};
use mml::MimeInterpreterBuilder;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{account::config::AccountConfig, email::address, message::Message, Result};

use super::Error;

/// Regex used to trim out prefix(es) from a subject.
///
/// Everything starting by "Re:" (case and whitespace insensitive) is
/// considered a prefix.
const PREFIXLESS_SUBJECT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i:\\s*re\\s*:\\s*)*(.*)").unwrap());

/// Trim out prefix(es) from the given subject.
fn prefixless_subject(subject: &str) -> &str {
    let cap = PREFIXLESS_SUBJECT_REGEX
        .captures(subject)
        .and_then(|cap| cap.get(1));

    match cap {
        Some(prefixless_subject) => prefixless_subject.as_str(),
        None => {
            warn!("cannot remove prefix from subject {subject:?}");
            subject
        }
    }
}

/// The message reply template builder.
///
/// This builder helps you to create a template in order to reply to
/// an existing message.
pub struct ReplyTplBuilder<'a> {
    /// Reference to the current account configuration.
    config: &'a AccountConfig,

    /// Reference to the original message.
    msg: &'a Message<'a>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Should reply to all.
    reply_all: bool,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,

    /// Template interpreter instance dedicated to the message thread.
    pub thread_interpreter: MimeInterpreterBuilder,
}

impl<'a> ReplyTplBuilder<'a> {
    /// Creates a reply template builder from an account configuration
    /// and a message references.
    pub fn new(msg: &'a Message, config: &'a AccountConfig) -> Self {
        Self {
            config,
            msg,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .with_show_only_headers(config.email_writing_headers()),
            thread_interpreter: config
                .generate_tpl_interpreter()
                .with_hide_all_headers()
                .with_show_plain_texts_signature(false)
                .with_show_attachments(false),
            reply_all: false,
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

    /// Sets the template interpreter following the builder pattern.
    pub fn with_interpreter(mut self, interpreter: MimeInterpreterBuilder) -> Self {
        self.interpreter = interpreter;
        self
    }

    /// Sets the template thread interpreter following the builder
    /// pattern.
    pub fn with_thread_interpreter(mut self, interpreter: MimeInterpreterBuilder) -> Self {
        self.thread_interpreter = interpreter;
        self
    }

    /// Sets the reply all flag following the builder pattern.
    pub fn with_reply_all(mut self, all: bool) -> Self {
        self.reply_all = all;
        self
    }

    /// Builds the final reply message template.
    pub async fn build(self) -> Result<String> {
        let parsed = self.msg.parsed()?;
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

        builder = builder.from(self.config.from());

        // To

        let recipients = if address::equal(&sender, &to) {
            // when replying to an email received by a mailing list
            if address::is_empty(&reply_to) {
                to.clone()
            } else {
                reply_to.clone()
            }
        } else if address::equal(
            &from,
            &HeaderValue::Address(mail_parser::Address::List(vec![me.clone()])),
        ) {
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
                    HeaderValue::Address(mail_parser::Address::List(addrs)) => {
                        for a in addrs {
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
                    HeaderValue::Address(mail_parser::Address::List(addrs)) => {
                        for a in addrs {
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

        // TODO: make this customizable?
        let prefix = String::from("Re: ");
        let subject = prefixless_subject(parsed.subject().unwrap_or_default());

        builder = builder.subject(prefix + subject);

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
                .build()
                .from_msg(&parsed)
                .await
                .map_err(Error::InterpretMessageAsThreadTemplateError)?;

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
            .build()
            .from_msg_builder(builder)
            .await
            .map_err(Error::InterpretMessageAsTemplateError)?;

        Ok(tpl)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn prefixless_subject() {
        assert_eq!(super::prefixless_subject("Hello, world!"), "Hello, world!");
        assert_eq!(
            super::prefixless_subject("re:Hello, world!"),
            "Hello, world!"
        );
        assert_eq!(
            super::prefixless_subject("Re   :Hello, world!"),
            "Hello, world!"
        );
        assert_eq!(
            super::prefixless_subject("rE:   Hello, world!"),
            "Hello, world!"
        );
        assert_eq!(
            super::prefixless_subject("  RE:  re  :Hello, world!"),
            "Hello, world!"
        );
    }
}
