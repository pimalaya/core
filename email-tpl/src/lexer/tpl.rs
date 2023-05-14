use lettre::message::{Mailboxes, Message, MultiPart, SinglePart};
use log::warn;
use std::collections::HashMap;
use thiserror::Error;

use crate::{evaluator::CompilerOpts, Result};

use super::part::{Part, PartKind};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build multi part")]
    BuildMultiPartError(#[source] lettre::error::Error),
    #[error("cannot build single part")]
    BuildSinglePartError(#[source] lettre::error::Error),
}

pub(crate) type Key = String;
pub(crate) type Val = String;
pub(crate) type Headers = HashMap<Key, Val>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Tpl {
    headers: Headers,
    parts: Vec<Part>,
}

impl Tpl {
    pub(crate) fn compile(self, mut opts: CompilerOpts) -> Result<Vec<u8>> {
        let mut builder = Message::builder().message_id(None);

        for (key, val) in &self.headers {
            builder = match key.to_lowercase().as_str() {
                "message-id" => builder.message_id(Some(val.clone())),
                "in-reply-to" => builder.in_reply_to(val.clone()),
                "subject" => builder.subject(val),
                "from" => match val.parse::<Mailboxes>() {
                    Ok(mboxes) => {
                        for mbox in mboxes {
                            builder = builder.from(mbox);
                        }
                        builder
                    }
                    Err(err) => {
                        warn!("skipping invalid sender address {}: {}", val, err);
                        builder
                    }
                },
                "to" => match val.parse::<Mailboxes>() {
                    Ok(mboxes) => {
                        for mbox in mboxes {
                            if let None = opts.pgp_encrypt_recipient {
                                opts.pgp_encrypt_recipient = Some(mbox.email.clone());
                            };
                            builder = builder.to(mbox);
                        }
                        builder
                    }
                    Err(err) => {
                        warn!("skipping invalid recipient address {}: {}", val, err);
                        builder
                    }
                },
                "reply-to" => match val.parse::<Mailboxes>() {
                    Ok(mboxes) => {
                        for mbox in mboxes {
                            builder = builder.reply_to(mbox);
                        }
                        builder
                    }
                    Err(err) => {
                        warn!("skipping invalid reply to address {}: {}", val, err);
                        builder
                    }
                },
                "cc" => match val.parse::<Mailboxes>() {
                    Ok(mboxes) => {
                        for mbox in mboxes {
                            builder = builder.cc(mbox);
                        }
                        builder
                    }
                    Err(err) => {
                        warn!("skipping invalid cc address {}: {}", val, err);
                        builder
                    }
                },
                "bcc" => match val.parse::<Mailboxes>() {
                    Ok(mboxes) => {
                        for mbox in mboxes {
                            builder = builder.bcc(mbox);
                        }
                        builder
                    }
                    Err(err) => {
                        warn!("skipping invalid bcc address {}: {}", val, err);
                        builder
                    }
                },
                _ => {
                    warn!("skipping unknown header {}", val);
                    builder
                }
            };
        }

        let email = match self.parts.len() {
            0 => builder
                .singlepart(SinglePart::plain(String::new()))
                .map_err(Error::BuildSinglePartError),
            1 => match self.parts[0].compile(&opts)? {
                PartKind::Single(part) => builder
                    .singlepart(part)
                    .map_err(Error::BuildSinglePartError),
                PartKind::Multi(part) => {
                    builder.multipart(part).map_err(Error::BuildMultiPartError)
                }
            },
            _ => {
                let mut multipart = MultiPart::mixed().build();

                for part in Part::compact_text_plain_parts(self.parts) {
                    multipart = match part.compile(&opts)? {
                        PartKind::Single(part) => multipart.singlepart(part),
                        PartKind::Multi(part) => multipart.multipart(part),
                    };
                }

                builder
                    .multipart(multipart)
                    .map_err(Error::BuildMultiPartError)
            }
        }?;

        Ok(email.formatted())
    }
}

impl From<(Headers, Vec<Part>)> for Tpl {
    fn from((headers, parts): (Headers, Vec<Part>)) -> Self {
        Self { headers, parts }
    }
}
