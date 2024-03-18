//! Module dedicated to email message reply template.
//!
//! The main structure of this module is the [ReplyTplBuilder], which
//! helps you to build template in order to reply to a message.

pub mod config;

use log::debug;
use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mail_parser::{Addr, HeaderValue};
use mml::MimeInterpreterBuilder;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

use crate::{account::config::AccountConfig, email::address, message::Message, Result};

use self::config::{ReplyTemplateQuotePlacement, ReplyTemplateSignaturePlacement};

use super::{Error, Template};

/// Regex used to trim out prefix(es) from a subject.
///
/// Everything starting by "Re:" (case and whitespace insensitive) is
/// considered a prefix.
static PREFIXLESS_SUBJECT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i:\\s*re\\s*:\\s*)*(.*)").unwrap());

/// Regex used to detect if an email address is a noreply one.
///
/// Matches usual names like `no_reply`, `noreply`, but also
/// `do-not.reply`.
static NO_REPLY: Lazy<Regex> = Lazy::new(|| Regex::new("(?i:not?[_\\-\\.]?reply)").unwrap());

/// Trim out prefix(es) from the given subject.
fn prefixless_subject(subject: &str) -> &str {
    let cap = PREFIXLESS_SUBJECT_REGEX
        .captures(subject)
        .and_then(|cap| cap.get(1));

    match cap {
        Some(prefixless_subject) => prefixless_subject.as_str(),
        None => {
            debug!("cannot remove prefix from subject {subject:?}");
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
    config: Arc<AccountConfig>,

    /// Reference to the original message.
    msg: &'a Message<'a>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Should reply to all.
    reply_all: bool,

    /// Override the placement of the signature.
    ///
    /// Uses the signature placement from the account configuration if
    /// this one is `None`.
    signature_placement: Option<ReplyTemplateSignaturePlacement>,

    /// Override the placement of the quote.
    ///
    /// Uses the quote placement from the account configuration if
    /// this one is `None`.
    quote_placement: Option<ReplyTemplateQuotePlacement>,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,

    /// Template interpreter instance dedicated to the message thread.
    pub thread_interpreter: MimeInterpreterBuilder,
}

impl<'a> ReplyTplBuilder<'a> {
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
            signature_placement: None,
            quote_placement: None,
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

    /// Set some signature placement.
    pub fn set_some_signature_placement(
        &mut self,
        placement: Option<impl Into<ReplyTemplateSignaturePlacement>>,
    ) {
        self.signature_placement = placement.map(Into::into);
    }

    /// Set the signature placement.
    pub fn set_signature_placement(
        &mut self,
        placement: impl Into<ReplyTemplateSignaturePlacement>,
    ) {
        self.set_some_signature_placement(Some(placement));
    }

    /// Set some signature placement, using the builder pattern.
    pub fn with_some_signature_placement(
        mut self,
        placement: Option<impl Into<ReplyTemplateSignaturePlacement>>,
    ) -> Self {
        self.set_some_signature_placement(placement);
        self
    }

    /// Set the signature placement, using the builder pattern.
    pub fn with_signature_placement(
        mut self,
        placement: impl Into<ReplyTemplateSignaturePlacement>,
    ) -> Self {
        self.set_signature_placement(placement);
        self
    }

    /// Set some quote placement.
    pub fn set_some_quote_placement(
        &mut self,
        placement: Option<impl Into<ReplyTemplateQuotePlacement>>,
    ) {
        self.quote_placement = placement.map(Into::into);
    }

    /// Set the quote placement.
    pub fn set_quote_placement(&mut self, placement: impl Into<ReplyTemplateQuotePlacement>) {
        self.set_some_quote_placement(Some(placement));
    }

    /// Set some quote placement, using the builder pattern.
    pub fn with_some_quote_placement(
        mut self,
        placement: Option<impl Into<ReplyTemplateQuotePlacement>>,
    ) -> Self {
        self.set_some_quote_placement(placement);
        self
    }

    /// Set the quote placement, using the builder pattern.
    pub fn with_quote_placement(
        mut self,
        placement: impl Into<ReplyTemplateQuotePlacement>,
    ) -> Self {
        self.set_quote_placement(placement);
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
    pub async fn build(self) -> Result<Template> {
        let mut cursor = 0;

        let parsed = self.msg.parsed()?;
        let mut builder = MessageBuilder::new();

        let me = Addr::new(Some(&self.config.name), &self.config.email);

        let sender = parsed.header("Sender").unwrap_or(&HeaderValue::Empty);
        let from = parsed.header("From").unwrap_or(&HeaderValue::Empty);
        let to = parsed.header("To").unwrap_or(&HeaderValue::Empty);
        let reply_to = parsed.header("Reply-To").unwrap_or(&HeaderValue::Empty);

        let sig = self.config.find_full_signature();
        let sig_placement = self
            .signature_placement
            .unwrap_or_else(|| self.config.get_reply_tpl_signature_placement());
        let quote_placement = self
            .quote_placement
            .unwrap_or_else(|| self.config.get_reply_tpl_quote_placement());
        let quote_headline = self.config.get_reply_tpl_quote_headline(parsed);

        // In-Reply-To

        match parsed.header("Message-ID") {
            Some(HeaderValue::Text(message_id)) => {
                builder = builder.in_reply_to(vec![message_id.clone()]);
                cursor += 1;
            }
            Some(HeaderValue::TextList(message_id)) => {
                builder = builder.in_reply_to(message_id.clone());
                cursor += 1;
            }
            _ => (),
        }

        // From

        builder = builder.from(self.config.as_ref());
        cursor += 1;

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
        cursor += 1;

        // Cc

        let cc = {
            let mut addresses = Vec::new();

            if !i_am_a_main_recipient && address::is_empty(&reply_to) {
                if !address::is_empty(&from) {
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
                } else {
                    if let HeaderValue::Address(mail_parser::Address::List(addrs)) = &sender {
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
            cursor += 1;
        }

        // Subject

        // TODO: make this customizable?
        let prefix = String::from("Re: ");
        let subject = prefixless_subject(parsed.subject().unwrap_or_default());

        builder = builder.subject(prefix + subject);
        cursor += 1;

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
            cursor += 1;
        }

        // Body

        builder = builder.text_body({
            let mut lines = String::from("\n\n");
            cursor += 2;

            let body = self
                .thread_interpreter
                .build()
                .from_msg(parsed)
                .await
                .map_err(Error::InterpretMessageAsThreadTemplateError)?;

            if quote_placement.is_above_reply() {
                if let Some(ref headline) = quote_headline {
                    lines.push_str(headline);
                    cursor += headline.lines().count();
                }

                for line in body.trim().lines() {
                    cursor += 1;
                    lines.push('>');
                    if !line.starts_with('>') {
                        lines.push(' ')
                    }
                    lines.push_str(line);
                    lines.push('\n');
                }
            }

            if !self.body.is_empty() {
                lines.push_str(&self.body);
                lines.push('\n');
            }

            if sig_placement.is_above_quote() {
                if let Some(ref sig) = sig {
                    lines.push('\n');
                    lines.push_str(sig);
                }
            }

            if quote_placement.is_below_reply() {
                if let Some(ref headline) = quote_headline {
                    lines.push_str(headline);
                }

                for line in body.trim().lines() {
                    lines.push('>');
                    if !line.starts_with('>') {
                        lines.push(' ')
                    }
                    lines.push_str(line);
                    lines.push('\n');
                }
            }

            if sig_placement.is_below_quote() {
                if let Some(ref sig) = sig {
                    lines.push('\n');
                    lines.push_str(sig);
                }
            }

            lines.trim_end().to_owned()
        });

        if sig_placement.is_attached() {
            if let Some(sig) = sig {
                builder = builder.attachment("text/plain", "signature.txt", sig)
            }
        }

        let tpl = self
            .interpreter
            .build()
            .from_msg_builder(builder)
            .await
            .map_err(Error::InterpretMessageAsTemplateError)?;

        Ok(Template::new(tpl))
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
