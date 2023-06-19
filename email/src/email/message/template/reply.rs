use mail_builder::{
    headers::{address::Address, raw::Raw},
    MessageBuilder,
};
use mail_parser::{Addr, HeaderValue};
use pimalaya_email_tpl::{Tpl, TplInterpreter};

use crate::{email::address, AccountConfig, Message, Result};

use super::Error;

pub struct ReplyTplBuilder<'a> {
    msg: &'a Message<'a>,
    config: &'a AccountConfig,
    headers: Vec<(String, String)>,
    body: String,
    pub thread_interpreter: TplInterpreter,
    pub interpreter: TplInterpreter,
    reply_all: bool,
}

impl<'a> ReplyTplBuilder<'a> {
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
                .hide_all_headers()
                .show_plain_texts_signature(false)
                .show_attachments(false),
            reply_all: false,
        }
    }

    pub fn headers<K: ToString, V: ToString>(
        mut self,
        headers: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        self.headers.extend(
            headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        );
        self
    }

    pub fn some_headers<K: ToString, V: ToString>(
        mut self,
        headers: Option<impl IntoIterator<Item = (K, V)>>,
    ) -> Self {
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
        } else if address::equal(&from, &HeaderValue::Address(me.clone())) {
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
                    HeaderValue::Address(a) => {
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
                    HeaderValue::AddressList(a) => {
                        for a in a {
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
                    HeaderValue::Address(a) => {
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
                    HeaderValue::AddressList(a) => {
                        for a in a {
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

        let subject = parsed
            .header("Subject")
            .cloned()
            .map(|h| h.unwrap_text())
            .unwrap_or_default();

        builder = builder.subject(if subject.to_lowercase().starts_with("re:") {
            subject
        } else {
            format!("Re: {subject}").into()
        });

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
                .interpret_msg(&parsed)
                .map_err(Error::InterpretEmailAsTplError)?;

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
            .interpret_msg_builder(builder)
            .map_err(Error::InterpretEmailAsTplError)?;

        Ok(tpl)
    }
}
