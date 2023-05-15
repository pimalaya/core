use lettre::Address;
use log::warn;
use pimalaya_process::Cmd;
use thiserror::Error;

use crate::{
    parser::{self, prelude::*},
    Result,
};

#[derive(Debug, Error)]
pub enum Error {
    // TODO: return the original chumsky::Error
    #[error("cannot parse template: {0}")]
    ParseTplError(String),
    #[error("cannot compile template: recipient is missing")]
    CompileTplMissingRecipientError,
}

/// Represents the compiler builder. It allows you to customize the
/// template compilation using the [Builder pattern].
///
/// [Builder pattern]: https://en.wikipedia.org/wiki/Builder_pattern
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompilerBuilder {
    /// Represents the PGP encrypt system command. Defaults to `gpg
    /// --encrypt --armor --recipient <recipient> --quiet --output -`.
    pgp_encrypt_cmd: Option<Cmd>,

    /// Represents the PGP encrypt recipient. By default, it will take
    /// the first address found from the "To" header of the template
    /// being compiled.
    pgp_encrypt_recipient: Option<Address>,

    /// Represents the PGP sign system command. Defaults to `gpg
    /// --sign --armor --quiet --output -`.
    pgp_sign_cmd: Option<Cmd>,
}

impl CompilerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pgp_encrypt_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_encrypt_cmd = Some(cmd.into());
        self
    }

    pub fn some_pgp_encrypt_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.pgp_encrypt_cmd = cmd.map(|c| c.into());
        self
    }

    pub fn pgp_encrypt_recipient<R: AsRef<str>>(mut self, recipient: R) -> Self {
        match recipient.as_ref().parse() {
            Ok(mbox) => {
                self.pgp_encrypt_recipient = Some(mbox);
            }
            Err(err) => {
                warn!(
                    "skipping invalid pgp encrypt recipient {}: {}",
                    recipient.as_ref(),
                    err
                );
            }
        }
        self
    }

    pub fn pgp_sign_cmd<C: Into<Cmd>>(mut self, cmd: C) -> Self {
        self.pgp_sign_cmd = Some(cmd.into());
        self
    }

    pub fn some_pgp_sign_cmd<C: Into<Cmd>>(mut self, cmd: Option<C>) -> Self {
        self.pgp_sign_cmd = cmd.map(|c| c.into());
        self
    }

    /// Compiles the given string template into a raw MIME Message
    /// using [CompilerOpts] from the builder.
    pub fn compile<T: AsRef<str>>(&self, tpl: T) -> Result<Vec<u8>> {
        let mime_msg = parser::tpl()
            .parse(tpl.as_ref())
            .map_err(|errs| Error::ParseTplError(errs[0].to_string()))?
            .compile(CompilerOpts {
                pgp_encrypt_cmd: self.pgp_sign_cmd.clone().unwrap_or_else(|| {
                    "gpg --encrypt --armor --recipient <recipient> --quiet --output -".into()
                }),
                pgp_encrypt_recipient: self.pgp_encrypt_recipient.clone(),
                pgp_sign_cmd: self
                    .pgp_sign_cmd
                    .clone()
                    .unwrap_or_else(|| "gpg --sign --armor --quiet --output -".into()),
            })?;

        Ok(mime_msg)
    }
}

/// Represents the compiler options. It is the final struct passed
/// down to the [Tpl::compile] function.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompilerOpts {
    pub pgp_encrypt_cmd: Cmd,
    pub pgp_encrypt_recipient: Option<Address>,
    pub pgp_sign_cmd: Cmd,
}

impl CompilerOpts {
    /// Builds the final PGP encrypt syste command by replacing
    /// `<recipient>` occurrences with the actual recipient. Fails in
    /// case no recipient is found.
    pub(crate) fn pgp_encrypt_cmd(&self) -> Result<Cmd> {
        let recipient = self
            .pgp_encrypt_recipient
            .as_ref()
            .ok_or(Error::CompileTplMissingRecipientError)?;

        let cmd = self
            .pgp_encrypt_cmd
            .clone()
            .replace("<recipient>", &recipient.to_string());

        Ok(cmd)
    }
}
