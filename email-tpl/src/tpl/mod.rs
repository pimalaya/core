pub(crate) mod header;
pub mod interpreter;

pub use interpreter::{Interpreter as TplInterpreter, ShowHeadersStrategy};

use mail_builder::{headers::raw::Raw, MessageBuilder};
use mail_parser::{Addr, Group, HeaderValue, Message};
use std::{
    io,
    ops::{Deref, DerefMut},
};
use thiserror::Error;

use crate::{mml, Encrypt, Result, Sign};

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
    /// Inner reference to the [MML compiler](crate::mml::Compiler).
    mml_compiler: mml::Compiler,

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

    pub fn with_encrypt(mut self, encrypt: Encrypt) -> Self {
        self.mml_compiler = self.mml_compiler.with_encrypt(encrypt);
        self
    }

    pub fn with_sign(mut self, sign: Sign) -> Self {
        self.mml_compiler = self.mml_compiler.with_sign(sign);
        self
    }

    pub async fn compile<'a>(self) -> Result<MessageBuilder<'a>> {
        let tpl = Message::parse(self.as_bytes()).ok_or(Error::ParseMessageError)?;

        let sender = extract_first_email_from_header(tpl.from());

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

        let mut builder = self.mml_compiler.compile(&mml).await?;

        builder = builder.header("MIME-Version", Raw::new("1.0"));

        for (key, val) in tpl.headers_raw() {
            let key = key.trim().to_owned();
            let val = Raw::new(val.trim().to_owned());
            builder = builder.header(key, val);
        }

        Ok(builder)
    }
}

fn extract_email_from_addr(a: &Addr) -> Option<String> {
    a.address.as_ref().map(|a| a.to_string())
}

fn extract_first_email_from_addrs(a: &Vec<Addr>) -> Option<String> {
    a.iter().next().and_then(extract_email_from_addr)
}

fn extract_first_email_from_group(g: &Group) -> Option<String> {
    extract_first_email_from_addrs(&g.addresses)
}

fn extract_first_email_from_groups(g: &Vec<Group>) -> Option<String> {
    g.first()
        .map(|g| &g.addresses)
        .and_then(extract_first_email_from_addrs)
}

fn extract_first_email_from_header(h: &HeaderValue) -> Option<String> {
    match h {
        HeaderValue::Address(a) => extract_email_from_addr(a),
        HeaderValue::AddressList(a) => extract_first_email_from_addrs(a),
        HeaderValue::Group(g) => extract_first_email_from_group(g),
        HeaderValue::GroupList(g) => extract_first_email_from_groups(g),
        _ => None,
    }
}
