pub(crate) mod header;
pub mod interpreter;

pub use interpreter::{Interpreter as TplInterpreter, ShowHeadersStrategy};

use log::{debug, warn};
use mail_builder::{headers::raw::Raw, MessageBuilder};
use mail_parser::Message;
use pimalaya_keyring::Entry;
use std::{
    io,
    ops::{Deref, DerefMut},
    path::PathBuf,
};
use thiserror::Error;

use crate::{mml, Result};

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
    #[error("cannot parse empty sender")]
    ParseSenderEmptyError,

    #[error("cannot get pgp secret key from keyring")]
    GetSecretKeyFromKeyringError(pimalaya_keyring::Error),
    #[error("cannot read pgp secret key from keyring")]
    ReadSecretKeyFromKeyringError(pimalaya_pgp::Error),
    #[error("cannot read pgp secret key from path {1}")]
    ReadSecretKeyFromPathError(pimalaya_pgp::Error, PathBuf),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Encrypt {
    #[default]
    None,
    KeyServers(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Sign {
    #[default]
    None,
    Path(String),
    Keyring(String),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Tpl {
    /// PGP encrypt configuration.
    pgp_encrypt: Encrypt,

    /// PGP sign configuration.
    pgp_sign: Sign,

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

    pub fn with_encrypt(mut self, encrypt: Encrypt) -> Self {
        self.pgp_encrypt = encrypt;
        self
    }

    pub fn with_sign(mut self, sign: Sign) -> Self {
        self.pgp_sign = sign;
        self
    }

    pub async fn compile<'a>(self) -> Result<MessageBuilder<'a>> {
        let tpl = Message::parse(self.as_bytes()).ok_or(Error::ParseMessageError)?;

        let sender = header::extract_first_email(tpl.from()).ok_or(Error::ParseSenderEmptyError)?;
        let recipients = header::extract_emails(tpl.to());

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
            .with_pgp_encrypt_keys(match &self.pgp_encrypt {
                Encrypt::None => {
                    warn!("cannot set pgp encrypt keys: encrypt not configured");
                    None
                }
                Encrypt::KeyServers(key_servers) => {
                    let wkd_pkeys = pimalaya_pgp::wkd::get_all(recipients).await;
                    let (mut pkeys, emails) = wkd_pkeys.into_iter().fold(
                        (Vec::default(), Vec::default()),
                        |(mut pkeys, mut emails), (email, res)| {
                            match res {
                                Ok(pkey) => {
                                    pkeys.push(pkey);
                                }
                                Err(err) => {
                                    warn!("cannot get public key of {email} using wkd: {err}");
                                    debug!("cannot get public key of {email} using wkd: {err:?}");
                                    emails.push(email)
                                }
                            }
                            (pkeys, emails)
                        },
                    );

                    let hkps_pkeys =
                        pimalaya_pgp::hkps::get_all(emails, key_servers.to_owned()).await;
                    let (hkps_pkeys, emails) = hkps_pkeys.into_iter().fold(
                        (Vec::default(), Vec::default()),
                        |(mut pkeys, mut emails), (email, res)| {
                            match res {
                                Ok(pkey) => {
                                    pkeys.push(pkey);
                                }
                                Err(err) => {
                                    warn!("cannot get public key of {email} using hkps: {err}");
                                    debug!("cannot get public key of {email} using hkps: {err:?}");
                                    emails.push(email)
                                }
                            }
                            (pkeys, emails)
                        },
                    );

                    if !emails.is_empty() {
                        let emails_len = emails.len();
                        let emails = emails.join(", ");
                        warn!("cannot get public key of {emails_len} emails");
                        debug!("cannot get public key of {emails_len} emails: {emails}");
                    }

                    pkeys.extend(hkps_pkeys);

                    Some(pkeys)
                }
            })
            .with_pgp_sign_key(match &self.pgp_sign {
                Sign::None => {
                    warn!("cannot set pgp sign key: sign not configured");
                    None
                }
                Sign::Path(path_tpl) => {
                    let get_skey = || async {
                        let path = PathBuf::from(path_tpl.replace("<sender>", &sender));
                        let skey = pimalaya_pgp::read_signed_secret_key_from_path(path.clone())
                            .await
                            .map_err(|err| Error::ReadSecretKeyFromPathError(err, path))?;
                        Result::Ok(skey)
                    };
                    match get_skey().await {
                        Ok(skey) => Some(skey),
                        Err(err) => {
                            warn!("cannot get pgp secret key from path: {err}");
                            debug!("cannot get pgp secret key from path: {err:?}");
                            None
                        }
                    }
                }
                Sign::Keyring(entry_tpl) => {
                    let get_skey = || async {
                        let data = Entry::from(entry_tpl.replace("<sender>", &sender))
                            .get_secret()
                            .map_err(Error::GetSecretKeyFromKeyringError)?;
                        let skey = pimalaya_pgp::read_skey_from_string(data)
                            .await
                            .map_err(Error::ReadSecretKeyFromKeyringError)?;
                        Result::Ok(skey)
                    };
                    match get_skey().await {
                        Ok(skey) => Some(skey),
                        Err(err) => {
                            warn!("cannot get pgp secret key from keyring: {err}");
                            debug!("cannot get pgp secret key from keyring: {err:?}");
                            None
                        }
                    }
                }
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
