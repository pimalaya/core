//! Module dedicated to email message forward template.
//!
//! The main structure of this module is the [ForwardTplBuilder],
//! which helps you to build template in order to forward a message.

use log::warn;
use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mml::MimeInterpreterBuilder;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{account::AccountConfig, boxed_err, email::Message, Result};

use super::Error;

/// Regex used to trim out prefix(es) from a subject.
///
/// Everything starting by "Fwd:" (case and whitespace insensitive) is
/// considered a prefix.
const PREFIXLESS_SUBJECT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new("(?i:\\s*fwd\\s*:\\s*)*(.*)").unwrap());

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
pub struct ForwardTplBuilder<'a> {
    /// Reference to the current account configuration.
    config: &'a AccountConfig,

    /// Reference to the original message.
    msg: &'a Message<'a>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,

    /// Template interpreter instance dedicated to the message thread.
    pub thread_interpreter: MimeInterpreterBuilder,
}

impl<'a> ForwardTplBuilder<'a> {
    /// Creates a forward template builder from an account
    /// configuration and a message references.
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
                .with_show_only_headers(["Date", "From", "To", "Cc", "Subject"])
                .with_save_attachments(true),
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

    /// Builds the final forward message template.
    pub async fn build(self) -> Result<String> {
        let parsed = self.msg.parsed()?;
        let mut builder = MessageBuilder::new();

        // From

        builder = builder.from(self.config.from());

        // To

        builder = builder.to(Vec::<Address>::new());

        // Subject

        // TODO: make this customizable?
        let prefix = String::from("Fwd: ");
        let subject = prefixless_subject(parsed.subject().unwrap_or_default());

        builder = builder.subject(prefix + subject);

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
                    .build()
                    .from_msg(&parsed)
                    .await
                    .map_err(|err| boxed_err(Error::InterpretMessageAsThreadTemplateError(err)))?,
            );

            lines.trim_end().to_owned()
        });

        let tpl = self
            .interpreter
            .build()
            .from_msg_builder(builder)
            .await
            .map_err(|err| boxed_err(Error::InterpretMessageAsTemplateError(err)))?;

        Ok(tpl)
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
