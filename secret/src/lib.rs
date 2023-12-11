//! Rust library to retrieve secrets from different sources.
//!
//! The core concept of this library is to abstract the concept of
//! secret. A secret can be retrieved either from a raw string, from a
//! command or from a keyring entry. The associated structure is
//! [`Secret`].

#[doc(inline)]
pub use keyring;
use keyring::Entry;
#[doc(inline)]
pub use process;
use process::Cmd;
use serde::{Deserialize, Serialize};
use std::result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get secret: secret is not defined")]
    GetSecretFromUndefinedError,
    #[error("cannot get secret from command")]
    GetSecretFromCmd(#[source] process::Error),
    #[error("cannot get secret from command: output is empty")]
    GetSecretFromCmdEmptyOutputError,

    #[error(transparent)]
    KeyringError(#[from] keyring::Error),
}

pub type Result<T> = result::Result<T, Error>;

/// The secret enum.
///
/// A secret can be retrieved either from a raw string, from a command
/// or from a keyring entry.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Secret {
    /// The secret is contained in a raw string, usually not safe to
    /// use and so not recommended.
    Raw(String),

    /// The secret is exposed by the given shell command.
    Cmd(Cmd),

    /// The secret is contained in the given user's global keyring at
    /// the given entry.
    #[serde(rename = "keyring")]
    KeyringEntry(Entry),

    /// The secret is not defined.
    #[default]
    Undefined,
}

impl Secret {
    /// Create a new secret from the given raw string.
    pub fn new_raw(raw: impl ToString) -> Self {
        Self::Raw(raw.to_string())
    }

    /// Create a new secret from the given shell command.
    pub fn new_cmd(cmd: impl Into<Cmd>) -> Self {
        Self::Cmd(cmd.into())
    }

    /// Create a new secret from the given keyring entry.
    pub fn new_keyring_entry(entry: impl Into<Entry>) -> Self {
        Self::KeyringEntry(entry.into())
    }

    /// Return `true` if the secret is not defined.
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Get the secret value.
    pub async fn get(&self) -> Result<String> {
        match self {
            Self::Raw(raw) => Ok(raw.clone()),
            Self::Cmd(cmd) => Ok(cmd
                .run()
                .await
                .map_err(Error::GetSecretFromCmd)?
                .to_string_lossy()
                .lines()
                .take(1)
                .next()
                .ok_or(Error::GetSecretFromCmdEmptyOutputError)?
                .to_owned()),
            Self::KeyringEntry(entry) => Ok(entry.get_secret().await?),
            Self::Undefined => Err(Error::GetSecretFromUndefinedError),
        }
    }

    /// Find the secret value.
    ///
    /// Return [`None`] if no secret is found.
    pub async fn find(&self) -> Result<Option<String>> {
        match self {
            Self::Raw(raw) => Ok(Some(raw.clone())),
            Self::Cmd(cmd) => Ok(cmd
                .run()
                .await
                .map_err(Error::GetSecretFromCmd)?
                .to_string_lossy()
                .lines()
                .take(1)
                .next()
                .map(ToOwned::to_owned)),
            Self::KeyringEntry(entry) => Ok(entry.find_secret().await?),
            Self::Undefined => Err(Error::GetSecretFromUndefinedError),
        }
    }

    /// Change the secret value if the source is a keyring entry.
    pub async fn set_keyring_entry_secret(&self, secret: impl AsRef<str>) -> Result<String> {
        if let Self::KeyringEntry(entry) = self {
            entry.set_secret(secret.as_ref()).await?;
        }

        Ok(secret.as_ref().to_string())
    }

    /// Transform an undefined secret into a keyring entry one.
    pub fn set_keyring_entry_if_undefined(&mut self, entry: impl Into<Entry>) {
        if self.is_undefined() {
            *self = Self::new_keyring_entry(entry)
        }
    }

    /// Delete the keyring entry secret.
    pub async fn delete_keyring_entry_secret(&self) -> Result<()> {
        if let Self::KeyringEntry(entry) = self {
            entry.delete_secret().await?;
        }

        Ok(())
    }
}
