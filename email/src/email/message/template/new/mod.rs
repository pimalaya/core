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
use std::sync::Arc;

use crate::{account::config::AccountConfig, Result};

use super::{Error, Template};

use self::config::NewTemplateSignaturePlacement;

/// The new template builder.
///
/// This builder helps you to create a template in order to compose a
/// new message from scratch.
pub struct NewTplBuilder {
    /// Account configuration reference.
    config: Arc<AccountConfig>,

    /// Additional headers to add at the top of the template.
    headers: Vec<(String, String)>,

    /// Default body to put in the template.
    body: String,

    /// Override the placement of the signature.
    ///
    /// Uses the signature placement from the account configuration if
    /// this one is `None`.
    signature_placement: Option<NewTemplateSignaturePlacement>,

    /// Template interpreter instance.
    pub interpreter: MimeInterpreterBuilder,
}

impl NewTplBuilder {
    /// Creates a new template builder from an account configuration.
    pub fn new(config: Arc<AccountConfig>) -> Self {
        let interpreter = config
            .generate_tpl_interpreter()
            .with_show_only_headers(config.get_message_write_headers());

        Self {
            config,
            headers: Vec::new(),
            body: String::new(),
            signature_placement: None,
            interpreter,
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
        placement: Option<impl Into<NewTemplateSignaturePlacement>>,
    ) {
        self.signature_placement = placement.map(Into::into);
    }

    /// Set the signature placement.
    pub fn set_signature_placement(&mut self, placement: impl Into<NewTemplateSignaturePlacement>) {
        self.set_some_signature_placement(Some(placement));
    }

    /// Set some signature placement, using the builder pattern.
    pub fn with_some_signature_placement(
        mut self,
        placement: Option<impl Into<NewTemplateSignaturePlacement>>,
    ) -> Self {
        self.set_some_signature_placement(placement);
        self
    }

    /// Set the signature placement, using the builder pattern.
    pub fn with_signature_placement(
        mut self,
        placement: impl Into<NewTemplateSignaturePlacement>,
    ) -> Self {
        self.set_signature_placement(placement);
        self
    }

    /// Sets the template interpreter following the builder pattern.
    pub fn with_interpreter(mut self, interpreter: MimeInterpreterBuilder) -> Self {
        self.interpreter = interpreter;
        self
    }

    /// Builds the final new message template.
    pub async fn build(self) -> Result<Template> {
        let mut cursor_x = 0;
        let mut cursor_y = 0;

        let sig = self.config.find_full_signature();
        let sig_placement = self
            .signature_placement
            .unwrap_or_else(|| self.config.get_new_tpl_signature_placement());

        let mut builder = MessageBuilder::new();

        builder = builder.from(self.config.as_ref());
        cursor_x += 1;

        builder = builder.to(Vec::<Address>::new());
        cursor_x += 1;

        builder = builder.subject("");
        cursor_x += 1;

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
            cursor_x += 1;
        }

        builder = builder.text_body({
            let mut lines = String::new();
            cursor_x += 1;

            let body = self.body.trim();
            if !body.is_empty() {
                lines.push_str(body);
                match body.rsplit_once('\n') {
                    Some((left, right)) => {
                        cursor_x += left.trim().lines().count();
                        cursor_y += right.len();
                    }
                    None => {
                        cursor_y += body.len();
                    }
                }
            }

            lines.push('\n');

            if sig_placement.is_inline() {
                if let Some(ref sig) = sig {
                    lines.push_str("\n");
                    lines.push_str(sig);
                }
            }

            lines
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

        Ok(Template::new_with_cursor(tpl, cursor_x, cursor_y))
    }
}

#[cfg(test)]
mod tests {
    use concat_with::concat_line;
    use std::sync::Arc;

    use crate::{
        account::config::AccountConfig,
        template::{
            config::TemplateConfig,
            new::{
                config::{NewTemplateConfig, NewTemplateSignaturePlacement},
                NewTplBuilder,
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
            NewTplBuilder::new(config).build().await.unwrap(),
            Template::new_with_cursor(
                concat_line!("From: Me <me@localhost>", "To: ", "Subject: ", "", "", ""),
                4,
                0,
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
            NewTplBuilder::new(config.clone())
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
                    "",
                    "",
                ),
                6,
                0,
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
            NewTplBuilder::new(config.clone())
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
                    "Hello, world!",
                    "",
                ),
                4,
                13,
            )
        );

        assert_eq!(
            NewTplBuilder::new(config.clone())
                .with_body("Hello\n,\nworld!")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "Hello",
                    ",",
                    "world!",
                    "",
                ),
                6,
                6,
            )
        );

        assert_eq!(
            NewTplBuilder::new(config.clone())
                .with_body("Hello\n,\nworld!\n")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "Hello",
                    ",",
                    "world!",
                    "",
                ),
                6,
                6,
            )
        );

        assert_eq!(
            NewTplBuilder::new(config)
                .with_body("Hello\n,\nworld!\n\n\n")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "Hello",
                    ",",
                    "world!",
                    "",
                ),
                6,
                6,
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
            NewTplBuilder::new(config.clone()).build().await.unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "",
                    "",
                    "-- ",
                    "signature",
                    "",
                ),
                4,
                0,
            )
        );

        assert_eq!(
            NewTplBuilder::new(config.clone())
                // force to hide the signature just for this builder
                .with_signature_placement(NewTemplateSignaturePlacement::Nowhere)
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!("From: Me <me@localhost>", "To: ", "Subject: ", "", "", ""),
                4,
                0,
            )
        );

        let config = Arc::new(AccountConfig {
            display_name: Some("Me".into()),
            email: "me@localhost".into(),
            signature_delim: Some("~~ \n\n".into()),
            signature: Some("signature\n\n\n".into()),
            template: Some(TemplateConfig {
                new: Some(NewTemplateConfig {
                    signature_placement: Some(NewTemplateSignaturePlacement::Nowhere),
                }),
                ..Default::default()
            }),
            ..Default::default()
        });

        assert_eq!(
            NewTplBuilder::new(config.clone()).build().await.unwrap(),
            Template::new_with_cursor(
                concat_line!("From: Me <me@localhost>", "To: ", "Subject: ", "", "", ""),
                4,
                0,
            )
        );

        assert_eq!(
            NewTplBuilder::new(config)
                // force to show the signature just for this builder
                .with_signature_placement(NewTemplateSignaturePlacement::Inline)
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
                    "~~ ",
                    "",
                    "signature",
                    "",
                ),
                4,
                0,
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
            NewTplBuilder::new(config.clone())
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
                    "Hello, world!",
                    "",
                    "-- ",
                    "signature",
                    "",
                ),
                4,
                13,
            )
        );

        assert_eq!(
            NewTplBuilder::new(config.clone())
                .with_body("\n\nHello, world!\n\n\n\n")
                .build()
                .await
                .unwrap(),
            Template::new_with_cursor(
                concat_line!(
                    "From: Me <me@localhost>",
                    "To: ",
                    "Subject: ",
                    "",
                    "Hello, world!",
                    "",
                    "-- ",
                    "signature",
                    "",
                ),
                4,
                13,
            )
        );
    }
}
