//! # New template
//!
//! The main structure of this module is the [`NewTemplateBuilder`],
//! which helps you to build template in order to compose a new
//! message from scratch.

pub mod config;

use std::sync::Arc;

use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mml::MimeInterpreterBuilder;

use self::config::NewTemplateSignatureStyle;
use super::{Template, TemplateBody, TemplateCursor};
use crate::{account::config::AccountConfig, email::error::Error};

/// The new template builder.
///
/// This builder helps you to create a template in order to compose a
/// new message from scratch.
pub struct NewTemplateBuilder {
    /// Account configuration reference.
    config: Arc<AccountConfig>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Override the style of the signature.
    ///
    /// Uses the signature style from the account configuration if
    /// this one is `None`.
    signature_style: Option<NewTemplateSignatureStyle>,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,
}

impl NewTemplateBuilder {
    /// Create a new template builder from an account configuration.
    pub fn new(config: Arc<AccountConfig>) -> Self {
        let interpreter = config
            .generate_tpl_interpreter()
            .with_show_only_headers(config.get_message_write_headers());

        Self {
            config,
            headers: Vec::new(),
            body: String::new(),
            signature_style: None,
            interpreter,
        }
    }

    /// Set additional template headers following the builder pattern.
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

    /// Set some additional template headers following the builder
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

    /// Set some signature style.
    pub fn set_some_signature_style(
        &mut self,
        style: Option<impl Into<NewTemplateSignatureStyle>>,
    ) {
        self.signature_style = style.map(Into::into);
    }

    /// Set the signature style.
    pub fn set_signature_style(&mut self, style: impl Into<NewTemplateSignatureStyle>) {
        self.set_some_signature_style(Some(style));
    }

    /// Set some signature style, using the builder pattern.
    pub fn with_some_signature_style(
        mut self,
        style: Option<impl Into<NewTemplateSignatureStyle>>,
    ) -> Self {
        self.set_some_signature_style(style);
        self
    }

    /// Set the signature style, using the builder pattern.
    pub fn with_signature_style(mut self, style: impl Into<NewTemplateSignatureStyle>) -> Self {
        self.set_signature_style(style);
        self
    }

    /// Set the template interpreter following the builder pattern.
    pub fn with_interpreter(mut self, interpreter: MimeInterpreterBuilder) -> Self {
        self.interpreter = interpreter;
        self
    }

    /// Build the final new message template.
    pub async fn build(self) -> Result<Template, Error> {
        let sig = self.config.find_full_signature();
        let sig_style = self
            .signature_style
            .unwrap_or_else(|| self.config.get_new_template_signature_style());

        let mut msg = MessageBuilder::default();
        let mut cursor = TemplateCursor::default();

        msg = msg.from(self.config.as_ref());
        cursor.row += 1;

        msg = msg.to(Vec::<Address>::new());
        cursor.row += 1;

        msg = msg.subject("");
        cursor.row += 1;

        for (key, val) in self.headers {
            msg = msg.header(key, Raw::new(val));
            cursor.row += 1;
        }

        msg = msg.text_body({
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

            cursor = body.cursor.clone();
            body
        });

        if sig_style.is_attached() {
            if let Some(sig) = sig {
                msg = msg.attachment("text/plain", "signature.txt", sig)
            }
        }

        let content = self
            .interpreter
            .build()
            .from_msg_builder(msg)
            .await
            .map_err(Error::InterpretMessageAsTemplateError)?;

        Ok(Template::new_with_cursor(content, cursor))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use concat_with::concat_line;

    use crate::{
        account::config::AccountConfig,
        template::{
            config::TemplateConfig,
            new::{
                config::{NewTemplateConfig, NewTemplateSignatureStyle},
                NewTemplateBuilder,
            },
            Template,
        },
    };

    #[tokio::test]
    async fn default() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..AccountConfig::default()
        });

        assert_eq!(
            NewTemplateBuilder::new(config).build().await.unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "", // cursor here
                ),
                (5, 0),
            ),
        );
    }

    #[tokio::test]
    async fn with_headers() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..AccountConfig::default()
        });

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                .with_headers([("In-Reply-To", ""), ("Cc", "")])
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "In-Reply-To: ",
                    "Cc: ",
                    "Subject: ",
                    "",
                    "", // cursor here
                ),
                (7, 0),
            )
        );
    }

    #[tokio::test]
    async fn with_body() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            ..AccountConfig::default()
        });

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                // with single line body
                .with_body("Hello, world!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "Hello, world!", // cursor here
                ),
                (5, 13),
            )
        );

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                // with multi lines body
                .with_body("\n\nHello\n,\nworld!\n\n!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "",
                    "",
                    "Hello",
                    ",",
                    "world!",
                    "",
                    "!", // cursor here
                ),
                (11, 1),
            )
        );
    }

    #[tokio::test]
    async fn with_signature() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature: Some("signature".into()),
            ..AccountConfig::default()
        });

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "", // cursor here
                    "",
                    "-- ",
                    "signature",
                ),
                (5, 0),
            )
        );

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                // force to hide the signature just for this builder
                .with_signature_style(NewTemplateSignatureStyle::Hidden)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "", // cursor here
                ),
                (5, 0),
            )
        );

        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature_delim: Some("~~ \n\n".into()),
            signature: Some("signature\n\n\n".into()),
            template: Some(TemplateConfig {
                new: Some(NewTemplateConfig {
                    signature_style: Some(NewTemplateSignatureStyle::Hidden),
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "", // cursor here
                ),
                (5, 0),
            )
        );

        assert_eq!(
            NewTemplateBuilder::new(config)
                // force to show the signature just for this builder
                .with_signature_style(NewTemplateSignatureStyle::Inlined)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "", // cursor here
                    "",
                    "~~ ",
                    "",
                    "signature",
                ),
                (5, 0),
            )
        );
    }

    #[tokio::test]
    async fn with_body_and_signature() {
        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature: Some("signature".into()),
            ..AccountConfig::default()
        });

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                .with_body("Hello, world!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "Hello, world!", // cursor here
                    "",
                    "-- ",
                    "signature",
                ),
                (5, 13),
            )
        );

        assert_eq!(
            NewTemplateBuilder::new(config.clone())
                .with_body("\n\nHello,\n\nworld\n\n!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "",
                    "",
                    "Hello,",
                    "",
                    "world",
                    "",
                    "!", // cursor
                    "",
                    "-- ",
                    "signature",
                ),
                (11, 1),
            )
        );
    }
}
