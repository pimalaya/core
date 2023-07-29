pub(crate) mod header;
pub mod interpreter;

pub use interpreter::{Interpreter as TplInterpreter, ShowHeadersStrategy};

use mail_builder::{headers::raw::Raw, MessageBuilder};
use mail_parser::Message;
use std::{
    io,
    ops::{Deref, DerefMut},
    path::PathBuf,
};
use thiserror::Error;

use crate::{mml, PgpPublicKeys, PgpSecretKey, Result};

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

    #[error("cannot get pgp secret key from keyring")]
    GetSecretKeyFromKeyringError(pimalaya_keyring::Error),
    #[error("cannot read pgp secret key from keyring")]
    ReadSecretKeyFromKeyringError(pimalaya_pgp::Error),
    #[error("cannot read pgp secret key from path {1}")]
    ReadSecretKeyFromPathError(pimalaya_pgp::Error, PathBuf),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tpl {
    /// PGP encrypt configuration.
    pgp_encrypt: PgpPublicKeys,

    /// PGP sign configuration.
    pgp_sign: PgpSecretKey,

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

    pub fn with_pgp_encrypt(mut self, encrypt: impl Into<PgpPublicKeys>) -> Self {
        self.pgp_encrypt = encrypt.into();
        self
    }

    pub fn with_pgp_sign(mut self, sign: impl Into<PgpSecretKey>) -> Self {
        self.pgp_sign = sign.into();
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

        let mut builder = mml::Compiler::new()
            .with_pgp_encrypt_keys(
                self.pgp_encrypt
                    .get_pkeys(header::extract_emails(tpl.to()))
                    .await,
            )
            .with_pgp_sign_key(match header::extract_first_email(tpl.from()) {
                Some(sender) => self.pgp_sign.get_skey(sender).await,
                None => None,
            })
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
