//! Rust library to retrieve secrets from different sources.
//!
//! The core concept of this library is to abstract the concept of
//! secret. A secret can be retrieved either from a raw string, from a
//! command or from a keyring. The associated structure is
//! [`Secret`]. The usage of this library should be restricted to the
//! pimalaya project, since the keyring scope is hard coded.

use keyring::Entry;
use log::{debug, trace, warn};
use process::Cmd;
use std::result;
use thiserror::Error;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get secret: secret is not defined")]
    GetSecretFromUndefinedError,
    #[error("cannot get secret from command")]
    GetSecretFromCmd(#[source] process::Error),

    #[error(transparent)]
    KeyringError(#[from] keyring::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The secret enum.
///
/// A secret can be retrieved either from a raw string, from a command
/// or from a keyring.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Secret {
    /// The secret is contained in a raw string, usually not safe to
    /// use and so not recommended.
    Raw(String),

    /// The secret is exposed by the given shell command.
    Cmd(Cmd),

    /// The secret is contained in the given user's global keyring
    /// entry.
    KeyringEntry(Entry),

    /// The secret is not defined.
    #[default]
    Undefined,
}

impl Secret {
    /// Create a new [`Secret`] from the given raw string.
    pub fn new_raw(raw: impl ToString) -> Self {
        Self::Raw(raw.to_string())
    }

    /// Create a new [`Secret`] from the given shell command.
    pub fn new_cmd(cmd: impl Into<Cmd>) -> Self {
        Self::Cmd(cmd.into())
    }

    /// Create a new [`Secret`] from the given keyring entry.
    pub fn new_keyring_entry(entry: impl Into<Entry>) -> Self {
        Self::KeyringEntry(entry.into())
    }

    /// Return `true` if the [`Secret`] is not defined.
    pub fn is_undefined(&self) -> bool {
        let is_undefined = matches!(self, Self::Undefined);
        trace!("is secret undefined: {is_undefined}");
        is_undefined
    }

    /// Get the secret value of the [`Secret`].
    pub async fn get(&self) -> Result<String> {
        debug!("getting secret");
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
                .unwrap()
                .to_owned()),
            Self::KeyringEntry(entry) => Ok(entry.get_secret()?),
            Self::Undefined => Err(Error::GetSecretFromUndefinedError),
        }
    }

    /// Find the secret value of the [`Secret`]. Return None if not
    /// found (mostly for the keyring entry variant).
    pub async fn find(&self) -> Result<Option<String>> {
        debug!("finding secret");
        match self {
            Self::Raw(raw) => Ok(Some(raw.clone())),
            Self::Cmd(cmd) => Ok(Some(
                cmd.run()
                    .await
                    .map_err(Error::GetSecretFromCmd)?
                    .to_string_lossy(),
            )),
            Self::KeyringEntry(entry) => Ok(entry.find_secret()?),
            Self::Undefined => Err(Error::GetSecretFromUndefinedError),
        }
    }

    /// (Re)set the keyring entry secret of the [`Secret`].
    pub fn set_keyring_entry_secret(&self, secret: impl AsRef<str>) -> Result<String> {
        debug!("setting keyring entry secret");

        if let Self::KeyringEntry(entry) = self {
            entry.set_secret(secret.as_ref())?;
        } else {
            warn!("secret not a keyring entry, skipping")
        }

        Ok(secret.as_ref().to_string())
    }

    /// Transform an undefined [`Secret`] into a keyring entry one,
    /// otherwise do nothing.
    pub fn set_keyring_entry_if_undefined(&mut self, entry: impl Into<Entry>) {
        debug!("replacing undefined secret by keyring entry");

        if let Self::Undefined = self {
            *self = Self::new_keyring_entry(entry)
        } else {
            warn!("secret is already defined, skipping")
        }
    }

    /// Delete the keyring entry secret of the [`Secret`].
    pub fn delete_keyring_entry_secret(&self) -> Result<()> {
        debug!("deleting keyring entry secret");

        if let Self::KeyringEntry(entry) = self {
            entry.delete_secret()?;
        } else {
            warn!("secret not a keyring entry, skipping")
        }

        Ok(())
    }
}
