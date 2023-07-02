pub(crate) mod header;
pub mod interpreter;

pub use interpreter::{Interpreter as TplInterpreter, ShowHeadersStrategy};

use mail_builder::{headers::raw::Raw, MessageBuilder};
use mail_parser::Message;
use pimalaya_process::Cmd;
use std::{
    io,
    ops::{Deref, DerefMut},
    result,
};
use thiserror::Error;

use crate::mml;

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

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tpl {
    /// Represents the PGP encrypt system command. Defaults to `gpg
    /// --encrypt --armor --recipient <recipient> --quiet --output -`.
    pgp_encrypt_cmd: Cmd,

    /// Represents the PGP encrypt recipient. By default, it will take
    /// the first address found from the "To" header of the template
    /// being compiled.
    pgp_encrypt_recipient: String,

    /// Represents the PGP sign system command. Defaults to `gpg
    /// --sign --armor --quiet --output -`.
    pgp_sign_cmd: Cmd,

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

    pub fn pgp_encrypt_cmd<C>(mut self, cmd: C) -> Self
    where
        C: Into<Cmd>,
    {
        self.mml_compiler = self.mml_compiler.pgp_encrypt_cmd(cmd);
        self
    }

    pub fn some_pgp_encrypt_cmd<C>(mut self, cmd: Option<C>) -> Self
    where
        C: Into<Cmd>,
    {
        self.mml_compiler = self.mml_compiler.some_pgp_encrypt_cmd(cmd);
        self
    }

    pub fn pgp_encrypt_recipient<R>(mut self, recipient: R) -> Self
    where
        R: ToString,
    {
        self.mml_compiler = self.mml_compiler.pgp_encrypt_recipient(recipient);
        self
    }

    pub fn pgp_sign_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self
    where
        C: Into<Cmd>,
    {
        self.mml_compiler = self.mml_compiler.pgp_sign_cmd(cmd);
        self
    }

    pub fn some_pgp_sign_cmd<C>(mut self, cmd: Option<C>) -> Self
    where
        C: Into<Cmd>,
    {
        self.mml_compiler = self.mml_compiler.some_pgp_sign_cmd(cmd);
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
            .compile(&mml)
            .await
            .map_err(Error::CompileMmlError)?;

        builder = builder.header("MIME-Version", Raw::new("1.0"));

        for (key, val) in tpl.headers_raw() {
            let key = key.trim().to_owned();
            let val = Raw::new(val.trim().to_owned());
            builder = builder.header(key, val);
        }

        Ok(builder)
    }
}
