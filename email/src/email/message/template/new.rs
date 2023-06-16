use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use pimalaya_email_tpl::{Tpl, TplInterpreter};

use crate::AccountConfig;

use super::{Error, Result};

pub struct NewTplBuilder<'a> {
    config: &'a AccountConfig,
    headers: Vec<(String, String)>,
    body: String,
    pub thread_interpreter: TplInterpreter,
    pub interpreter: TplInterpreter,
    reply_all: bool,
}

impl<'a> NewTplBuilder<'a> {
    pub fn new(config: &'a AccountConfig) -> Self {
        Self {
            config,
            headers: Vec::new(),
            body: String::new(),
            interpreter: config
                .generate_tpl_interpreter()
                .show_only_headers(config.email_writing_headers()),
            thread_interpreter: config
                .generate_tpl_interpreter()
                .hide_all_headers()
                .show_plain_texts_signature(false)
                .show_attachments(false),
            reply_all: false,
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

    pub fn reply_all(mut self, all: bool) -> Self {
        self.reply_all = all;
        self
    }

    pub fn build(self) -> Result<Tpl> {
        let mut builder = MessageBuilder::new()
            .from(self.config.addr())
            .to(Vec::<Address>::new())
            .subject("")
            .text_body({
                let mut lines = String::new();

                if !self.body.is_empty() {
                    lines.push_str(&self.body);
                    lines.push('\n');
                }

                if let Some(ref signature) = self.config.signature()? {
                    lines.push_str("\n\n");
                    lines.push_str(signature);
                }

                lines
            });

        // Additional headers

        for (key, val) in self.headers {
            builder = builder.header(key, Raw::new(val));
        }

        let tpl = self
            .interpreter
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }
}
