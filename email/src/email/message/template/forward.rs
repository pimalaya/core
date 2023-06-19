use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use pimalaya_email_tpl::{Tpl, TplInterpreter};

use crate::{AccountConfig, Message, Result};

use super::Error;

pub struct ForwardTplBuilder<'a> {
    msg: &'a Message<'a>,
    config: &'a AccountConfig,
    headers: Vec<(String, String)>,
    body: String,
    pub interpreter: TplInterpreter,
    pub thread_interpreter: TplInterpreter,
}

impl<'a> ForwardTplBuilder<'a> {
    pub fn new(msg: &'a Message, config: &'a AccountConfig) -> Self {
        Self {
            msg,
            config,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(config.email_writing_headers()),
            thread_interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(["Date", "From", "To", "Cc", "Subject"])
                .save_attachments(true),
        }
    }

    pub fn headers<K, V>(mut self, headers: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    pub fn some_headers<K, V>(mut self, headers: Option<impl IntoIterator<Item = (K, V)>>) -> Self
    where
        K: ToString,
        V: ToString,
    {
        if let Some(headers) = headers {
            self = self.headers(headers);
        }
        self
    }

    pub fn body(mut self, body: impl ToString) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn some_body(mut self, body: Option<impl ToString>) -> Self {
        if let Some(body) = body {
            self = self.body(body)
        }
        self
    }

    pub fn interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.interpreter = interpreter;
        self
    }

    pub fn thread_interpreter(mut self, interpreter: TplInterpreter) -> Self {
        self.thread_interpreter = interpreter;
        self
    }

    pub fn build(self) -> Result<Tpl> {
        let parsed = self.msg.parsed()?;
        let mut builder = MessageBuilder::new();

        // From

        builder = builder.from(self.config.from());

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
                    .interpret_msg(&parsed)
                    .map_err(Error::InterpretEmailAsTplError)?,
            );

            lines.trim_end().to_owned()
        });

        let tpl = self
            .interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }
}
