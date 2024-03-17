//! Module dedicated to email message new template.
//!
//! The main structure of this module is the [NewTplBuilder], which
//! helps you to build template in order to compose a new message.

pub mod config;

use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mml::MimeInterpreterBuilder;

use crate::{account::config::AccountConfig, Result};

use super::Error;

/// The message new template builder.
///
/// This builder helps you to create a template in order to compose a
/// new message from scratch.
pub struct NewTplBuilder<'a> {
    /// Account configuration reference.
    config: &'a AccountConfig,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,
}

impl<'a> NewTplBuilder<'a> {
    /// Creates a new template builder from an account configuration.
    pub fn new(config: &'a AccountConfig) -> Self {
        Self {
            config,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .with_show_only_headers(config.get_message_write_headers()),
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

    /// Builds the final new message template.
    pub async fn build(self) -> Result<String> {
        let mut builder = MessageBuilder::new()
            .from(self.config.clone())
            .to(Vec::<Address>::new())
            .subject("")
            .text_body({
                let mut lines = String::new();

                if !self.body.is_empty() {
                    lines.push_str(&self.body);
                    lines.push('\n');
                }

                if let Some(ref signature) = self.config.find_full_signature()? {
                    lines.push_str("\n\n");
                    lines.push_str(signature);
                }

                lines
            });

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
        }

        let tpl = self
            .interpreter
            .build()
            .from_msg_builder(builder)
            .await
            .map_err(Error::InterpretMessageAsTemplateError)?;

        Ok(tpl)
    }
}
