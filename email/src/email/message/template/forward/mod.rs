//! # Forward template
//!
//! The main structure of this module is the
//! [`ForwardTemplateBuilder`], which helps you to build template in
//! order to forward a message.

pub mod config;

use std::sync::Arc;

use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mml::MimeInterpreterBuilder;
use once_cell::sync::Lazy;
use regex::Regex;

use self::config::{ForwardTemplatePostingStyle, ForwardTemplateSignatureStyle};
use super::{Template, TemplateBody, TemplateCursor};
use crate::{account::config::AccountConfig, email::error::Error, message::Message};

/// Regex used to trim out prefix(es) from a subject.
///
/// Everything starting by "Fwd:" (case and whitespace insensitive) is
/// considered a prefix.
static SUBJECT: Lazy<Regex> = Lazy::new(|| Regex::new("(?i:\\s*fwd\\s*:\\s*)*(.*)").unwrap());

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
pub struct ForwardTemplateBuilder<'a> {
    /// Reference to the current account configuration.
    config: Arc<AccountConfig>,

    /// Reference to the original message.
    msg: &'a Message<'a>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Override the placement of the quote.
    ///
    /// Uses the quote placement from the account configuration if
    /// this one is `None`.
    posting_style: Option<ForwardTemplatePostingStyle>,

    /// Override the placement of the signature.
    ///
    /// Uses the signature placement from the account configuration if
    /// this one is `None`.
    signature_style: Option<ForwardTemplateSignatureStyle>,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,

    /// Template interpreter instance dedicated to the message thread.
    pub thread_interpreter: MimeInterpreterBuilder,
}

impl<'a> ForwardTemplateBuilder<'a> {
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
            signature_style: None,
            posting_style: None,
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

    /// Set some forward posting style.
    pub fn set_some_posting_style(
        &mut self,
        style: Option<impl Into<ForwardTemplatePostingStyle>>,
    ) {
        self.posting_style = style.map(Into::into);
    }

    /// Set the forward posting style.
    pub fn set_posting_style(&mut self, style: impl Into<ForwardTemplatePostingStyle>) {
        self.set_some_posting_style(Some(style));
    }

    /// Set some forward posting style, using the builder pattern.
    pub fn with_some_posting_style(
        mut self,
        style: Option<impl Into<ForwardTemplatePostingStyle>>,
    ) -> Self {
        self.set_some_posting_style(style);
        self
    }

    /// Set the forward posting style, using the builder pattern.
    pub fn with_posting_style(mut self, style: impl Into<ForwardTemplatePostingStyle>) -> Self {
        self.set_posting_style(style);
        self
    }

    /// Set the signature style.
    pub fn set_signature_style(&mut self, style: impl Into<ForwardTemplateSignatureStyle>) {
        self.set_some_signature_style(Some(style));
    }

    /// Set some signature style.
    pub fn set_some_signature_style(
        &mut self,
        style: Option<impl Into<ForwardTemplateSignatureStyle>>,
    ) {
        self.signature_style = style.map(Into::into);
    }

    /// Set some signature style, using the builder pattern.
    pub fn with_some_signature_style(
        mut self,
        style: Option<impl Into<ForwardTemplateSignatureStyle>>,
    ) -> Self {
        self.set_some_signature_style(style);
        self
    }

    /// Set the signature style, using the builder pattern.
    pub fn with_signature_style(mut self, style: impl Into<ForwardTemplateSignatureStyle>) -> Self {
        self.set_signature_style(style);
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
    pub async fn build(self) -> Result<Template, Error> {
        let mut cursor = TemplateCursor::default();

        let parsed = self.msg.parsed()?;
        let mut builder = MessageBuilder::new();

        // From

        builder = builder.from(self.config.as_ref());
        cursor.row += 1;

        // To

        builder = builder.to(Vec::<Address>::new());
        cursor.row += 1;

        // Subject

        // TODO: make this customizable?
        let prefix = String::from("Fwd: ");
        let subject = trim_prefix(parsed.subject().unwrap_or_default());

        builder = builder.subject(prefix + subject);
        cursor.row += 1;

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
            cursor.row += 1;
        }

        // Body

        let sig = self.config.find_full_signature();
        let sig_style = self
            .signature_style
            .unwrap_or_else(|| self.config.get_forward_template_signature_style());
        let posting_style = self
            .posting_style
            .unwrap_or_else(|| self.config.get_forward_template_posting_style());
        let quote_headline = self.config.get_forward_template_quote_headline();

        builder = builder.text_body({
            let mut body = TemplateBody::new(cursor);

            body.push_str(&self.body);
            body.flush();
            body.cursor.lock();

            if sig_style.is_inlined() {
                if let Some(ref sig) = sig {
                    body.push_str(sig);
                    body.flush();
                }
            }

            if posting_style.is_top() {
                body.push_str(&quote_headline);
                body.push_str(
                    self.thread_interpreter
                        .build()
                        .from_msg(parsed)
                        .await
                        .map_err(Error::InterpretMessageAsThreadTemplateError)?
                        .trim(),
                );
                body.flush()
            }

            cursor = body.cursor.clone();
            body
        });

        if sig_style.is_attached() {
            if let Some(sig) = sig {
                builder = builder.attachment("text/plain", "signature.txt", sig)
            }
        }

        if posting_style.is_attached() {
            let file_name = parsed.message_id().unwrap_or("message");
            builder = builder.attachment(
                "message/rfc822",
                format!("{file_name}.eml"),
                parsed.raw_message(),
            )
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
    use std::sync::Arc;

    use concat_with::concat_line;

    use super::ForwardTemplateBuilder;
    use crate::{account::config::AccountConfig, message::Message, template::Template};

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
            "Hello, world!",
            "",
        ));

        assert_eq!(
            ForwardTemplateBuilder::new(msg, config)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: Fwd: subject",
                    "",
                    "", // cursor here
                    "",
                    "-------- Forwarded Message --------",
                    "From: sender@localhost",
                    "To: me@localhost",
                    "Subject: subject",
                    "",
                    "Hello, world!",
                ),
                (5, 0),
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
            ForwardTemplateBuilder::new(msg, config)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: Fwd: subject",
                    "",
                    "", // cursor here
                    "",
                    "-- ",
                    "signature",
                    "",
                    "-------- Forwarded Message --------",
                    "From: sender@localhost",
                    "To: me@localhost",
                    "Subject: subject",
                    "",
                    "Hello, world!",
                ),
                (5, 0),
            ),
        );
    }

    #[test]
    fn trim_subject_prefix() {
        assert_eq!(super::trim_prefix("Hello, world!"), "Hello, world!");
        assert_eq!(super::trim_prefix("fwd:Hello, world!"), "Hello, world!");
        assert_eq!(super::trim_prefix("Fwd   :Hello, world!"), "Hello, world!");
        assert_eq!(super::trim_prefix("fWd:   Hello, world!"), "Hello, world!");
        assert_eq!(
            super::trim_prefix("  FWD:  fwd  :Hello, world!"),
            "Hello, world!"
        );
    }
}
