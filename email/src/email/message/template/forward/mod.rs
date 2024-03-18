//! Module dedicated to email message forward template.
//!
//! The main structure of this module is the [ForwardTplBuilder],
//! which helps you to build template in order to forward a message.

pub mod config;

use log::debug;
use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mml::MimeInterpreterBuilder;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::Arc;

use crate::{account::config::AccountConfig, message::Message, Result};

use self::config::{ForwardTemplateQuotePlacement, ForwardTemplateSignaturePlacement};

use super::{Error, Template};

/// Regex used to trim out prefix(es) from a subject.
///
/// Everything starting by "Fwd:" (case and whitespace insensitive) is
/// considered a prefix.
static PREFIXLESS_SUBJECT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i:\\s*fwd\\s*:\\s*)*(.*)").unwrap());

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
pub struct ForwardTplBuilder<'a> {
    /// Reference to the current account configuration.
    config: Arc<AccountConfig>,

    /// Reference to the original message.
    msg: &'a Message<'a>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Override the placement of the signature.
    ///
    /// Uses the signature placement from the account configuration if
    /// this one is `None`.
    signature_placement: Option<ForwardTemplateSignaturePlacement>,

    /// Override the placement of the quote.
    ///
    /// Uses the quote placement from the account configuration if
    /// this one is `None`.
    quote_placement: Option<ForwardTemplateQuotePlacement>,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,

    /// Template interpreter instance dedicated to the message thread.
    pub thread_interpreter: MimeInterpreterBuilder,
}

impl<'a> ForwardTplBuilder<'a> {
    /// Creates a forward template builder from an account
    /// configuration and a message references.

    pub fn new(msg: &'a Message, config: Arc<AccountConfig>) -> Self {
        let interpreter = config
            .generate_tpl_interpreter()
            .with_show_only_headers(config.get_message_write_headers());

        let thread_interpreter = config
            .generate_tpl_interpreter()
            .with_show_only_headers(["Date", "From", "To", "Cc", "Subject"])
            .with_save_attachments(true);

        Self {
            config,
            msg,
            headers: Vec::new(),
            body: String::new(),
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
        placement: Option<impl Into<ForwardTemplateSignaturePlacement>>,
    ) {
        self.signature_placement = placement.map(Into::into);
    }

    /// Set the signature placement.
    pub fn set_signature_placement(
        &mut self,
        placement: impl Into<ForwardTemplateSignaturePlacement>,
    ) {
        self.set_some_signature_placement(Some(placement));
    }

    /// Set some signature placement, using the builder pattern.
    pub fn with_some_signature_placement(
        mut self,
        placement: Option<impl Into<ForwardTemplateSignaturePlacement>>,
    ) -> Self {
        self.set_some_signature_placement(placement);
        self
    }

    /// Set the signature placement, using the builder pattern.
    pub fn with_signature_placement(
        mut self,
        placement: impl Into<ForwardTemplateSignaturePlacement>,
    ) -> Self {
        self.set_signature_placement(placement);
        self
    }

    /// Set some quote placement.
    pub fn set_some_quote_placement(
        &mut self,
        placement: Option<impl Into<ForwardTemplateQuotePlacement>>,
    ) {
        self.quote_placement = placement.map(Into::into);
    }

    /// Set the quote placement.
    pub fn set_quote_placement(&mut self, placement: impl Into<ForwardTemplateQuotePlacement>) {
        self.set_some_quote_placement(Some(placement));
    }

    /// Set some quote placement, using the builder pattern.
    pub fn with_some_quote_placement(
        mut self,
        placement: Option<impl Into<ForwardTemplateQuotePlacement>>,
    ) -> Self {
        self.set_some_quote_placement(placement);
        self
    }

    /// Set the quote placement, using the builder pattern.
    pub fn with_quote_placement(
        mut self,
        placement: impl Into<ForwardTemplateQuotePlacement>,
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

    /// Builds the final forward message template.
    pub async fn build(self) -> Result<Template> {
        let mut cursor = 0;

        let parsed = self.msg.parsed()?;
        let mut builder = MessageBuilder::new();

        // From

        builder = builder.from(self.config.as_ref());
        cursor += 1;

        // To

        builder = builder.to(Vec::<Address>::new());
        cursor += 1;

        // Subject

        // TODO: make this customizable?
        let prefix = String::from("Fwd: ");
        let subject = prefixless_subject(parsed.subject().unwrap_or_default());

        builder = builder.subject(prefix + subject);
        cursor += 1;

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
            cursor += 1;
        }

        // Body

        let sig = self.config.find_full_signature();
        let sig_placement = self
            .signature_placement
            .unwrap_or_else(|| self.config.get_forward_tpl_signature_placement());
        let quote_placement = self
            .quote_placement
            .unwrap_or_else(|| self.config.get_forward_tpl_quote_placement());
        let quote_headline = self.config.get_forward_tpl_quote_headline();

        builder = builder.text_body({
            let mut lines = String::from("\n");
            cursor += 1;

            if !self.body.is_empty() {
                lines.push('\n');
                cursor += 1;
                lines.push_str(&self.body);
                lines.push('\n');
            }

            if sig_placement.is_inline() {
                if let Some(ref sig) = sig {
                    lines.push('\n');
                    lines.push_str(sig);
                    lines.push('\n');
                }
            }

            if quote_placement.is_inline() {
                lines.push('\n');
                lines.push_str(&quote_headline);
                lines.push_str(
                    &self
                        .thread_interpreter
                        .build()
                        .from_msg(parsed)
                        .await
                        .map_err(Error::InterpretMessageAsThreadTemplateError)?,
                );
            }

            lines.trim_end().to_owned()
        });

        if sig_placement.is_attached() {
            if let Some(sig) = sig {
                builder = builder.attachment("text/plain", "signature.txt", sig)
            }
        }

        if quote_placement.is_attached() {
            let file_name = parsed.message_id().unwrap_or("message");
            builder = builder.attachment(
                "message/rfc822",
                format!("{file_name}.eml"),
                parsed.raw_message(),
            )
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
            super::prefixless_subject("fwd:Hello, world!"),
            "Hello, world!"
        );
        assert_eq!(
            super::prefixless_subject("Fwd   :Hello, world!"),
            "Hello, world!"
        );
        assert_eq!(
            super::prefixless_subject("fWd:   Hello, world!"),
            "Hello, world!"
        );
        assert_eq!(
            super::prefixless_subject("  FWD:  fwd  :Hello, world!"),
            "Hello, world!"
        );
    }
}
