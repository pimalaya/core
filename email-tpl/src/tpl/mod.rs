pub(crate) mod header;
pub mod interpreter;

pub use interpreter::{Interpreter as TplInterpreter, ShowHeadersStrategy};

use mail_builder::{headers::raw::Raw, MessageBuilder};
use mail_parser::Message;
use std::{
    io,
    ops::{Deref, DerefMut},
};
use thiserror::Error;

use crate::{mml, Pgp, Result};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build message from template")]
    CreateMessageBuilderError,
    #[error("cannot compile template")]
    WriteTplToStringError(#[source] io::Error),
    #[error("cannot compile template")]
    WriteTplToVecError(#[source] io::Error),
    #[error("cannot compile mime meta language")]
    CompileMmlError(#[source] mml::compiler::Error),
    #[error("cannot interpret email as a template")]
    InterpretError(#[source] mml::interpreter::Error),
    #[error("cannot parse template")]
    ParseMessageError,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tpl {
    mml_compiler: mml::Compiler,

    /// Inner template data.
    data: String,
}

impl Deref for Tpl {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for Tpl {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: ToString> From<T> for Tpl {
    fn from(tpl: T) -> Self {
        Self {
            data: tpl.to_string(),
            ..Default::default()
        }
    }
}

impl Into<String> for Tpl {
    fn into(self) -> String {
        self.data
    }
}

impl Tpl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_pgp(mut self, pgp: impl Into<Pgp>) -> Self {
        self.mml_compiler = self.mml_compiler.with_pgp(pgp.into());
        self
    }

    pub async fn compile<'a>(self) -> Result<MessageBuilder<'a>> {
        let tpl = Message::parse(self.as_bytes()).ok_or(Error::ParseMessageError)?;

        let mml = tpl
            .text_bodies()
            .into_iter()
            .filter_map(|part| part.text_contents())
            .fold(String::new(), |mut contents, content| {
                if !contents.is_empty() {
                    contents.push_str("\n\n");
                }
                contents.push_str(content.trim());
                contents
            });

        let mut builder = self
            .mml_compiler
            .clone()
            .with_pgp_recipients(header::extract_emails(tpl.to()))
            .with_pgp_sender(header::extract_first_email(tpl.from()))
            .compile(&mml)
            .await?;

        builder = builder.header("MIME-Version", Raw::new("1.0"));

        for (key, val) in tpl.headers_raw() {
            let key = key.trim().to_owned();
            let val = Raw::new(val.trim().to_owned());
            builder = builder.header(key, val);
        }

        Ok(builder)
    }
}
