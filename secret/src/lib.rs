//! # Secret
//!
//! The core concept of this library is to abstract the concept of
//! secret. A secret can be retrieved either from a raw string, from a
//! command or from a keyring entry. The associated structure is
//! [`Secret`].

#[cfg(feature = "keyring")]
pub use keyring;
#[cfg(feature = "keyring")]
use keyring::KeyringEntry;
use log::debug;
#[cfg(feature = "command")]
pub use process;
#[cfg(feature = "command")]
use process::Cmd;
use std::result;
use thiserror::Error;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get secret: secret is not defined")]
    GetSecretFromUndefinedError,
    #[cfg(feature = "command")]
    #[error("cannot get secret from command")]
    GetSecretFromCmd(#[source] process::Error),
    #[cfg(feature = "command")]
    #[error("cannot get secret from command: output is empty")]
    GetSecretFromCmdEmptyOutputError,
    #[cfg(feature = "keyring")]
    #[error("error while using secret from keyring")]
    KeyringError(#[source] keyring::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The secret.
///
/// A secret can be retrieved either from a raw string, from a shell
/// command or from a keyring entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum Secret {
    /// The secret is contained in a raw string, usually not safe to
    /// use and so not recommended.
    Raw(String),

    /// The secret is exposed by the given shell command.
    #[cfg(feature = "command")]
    Command(Cmd),

    /// The secret is contained in the given user's global keyring at
    /// the given entry.
    #[cfg(feature = "keyring")]
    #[cfg_attr(feature = "serde", serde(rename = "keyring"))]
    KeyringEntry(KeyringEntry),

    /// The secret is not defined.
    #[default]
    Undefined,
}

impl Secret {
    /// Create a new undefined secret.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a new secret from the given raw string.
    pub fn new_raw(raw: impl ToString) -> Self {
        Self::Raw(raw.to_string())
    }

    /// Create a new secret from the given shell command.
    #[cfg(feature = "command")]
    pub fn new_cmd(cmd: impl Into<Cmd>) -> Self {
        Self::Command(cmd.into())
    }

    /// Create a new secret from the given keyring entry.
    #[cfg(feature = "keyring")]
    pub fn new_keyring_entry(entry: KeyringEntry) -> Self {
        Self::KeyringEntry(entry)
    }

    /// Try to create a new secret from the given entry.
    #[cfg(feature = "keyring")]
    pub fn try_new_keyring_entry(
        entry: impl TryInto<KeyringEntry, Error = keyring::Error>,
    ) -> Result<Self> {
        let entry = entry.try_into().map_err(Error::KeyringError)?;
        Ok(Self::KeyringEntry(entry))
    }

    /// Return `true` if the secret is not defined.
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Get the secret.
    pub async fn get(&self) -> Result<String> {
        match self {
            Self::Raw(raw) => Ok(raw.clone()),
            #[cfg(feature = "command")]
            Self::Command(cmd) => Ok(cmd
                .run()
                .await
                .map_err(Error::GetSecretFromCmd)?
                .to_string_lossy()
                .lines()
                .take(1)
                .next()
                .ok_or(Error::GetSecretFromCmdEmptyOutputError)?
                .to_owned()),
            #[cfg(feature = "keyring")]
            Self::KeyringEntry(entry) => {
                Ok(entry.get_secret().await.map_err(Error::KeyringError)?)
            }
            Self::Undefined => Err(Error::GetSecretFromUndefinedError),
        }
    }

    /// Find the secret value.
    ///
    /// Return [`None`] if no secret is not found.
    pub async fn find(&self) -> Result<Option<String>> {
        match self {
            Self::Raw(secret) => Ok(Some(secret.clone())),
            #[cfg(feature = "command")]
            Self::Command(cmd) => Ok(cmd
                .run()
                .await
                .map_err(Error::GetSecretFromCmd)?
                .to_string_lossy()
                .lines()
                .take(1)
                .next()
                .map(ToOwned::to_owned)),
            #[cfg(feature = "keyring")]
            Self::KeyringEntry(entry) => {
                Ok(entry.find_secret().await.map_err(Error::KeyringError)?)
            }
            Self::Undefined => Ok(None),
        }
    }

    /// Change the secret.
    pub async fn set(&mut self, secret: impl AsRef<str>) -> Result<String> {
        let secret = secret.as_ref();

        match self {
            Self::Raw(prev) => {
                *prev = secret.to_owned();
            }
            #[cfg(feature = "command")]
            Self::Command(_) => {
                debug!("cannot change secret of command variant");
            }
            #[cfg(feature = "keyring")]
            Self::KeyringEntry(entry) => {
                entry
                    .set_secret(secret)
                    .await
                    .map_err(Error::KeyringError)?;
            }
            Self::Undefined => {
                debug!("cannot change secret of undefined variant");
            }
        }

        Ok(secret.to_owned())
    }

    /// Delete the secret.
    ///
    /// If the secret uses the keyring entry variant, delete the
    /// secret from the keyring. Otherwise change self to undefined
    /// variant.
    pub async fn delete(&mut self) -> Result<()> {
        match self {
            #[cfg(feature = "keyring")]
            Self::KeyringEntry(entry) => {
                entry.delete_secret().await.map_err(Error::KeyringError)?;
            }
            _ => {
                *self = Self::Undefined;
            }
        };

        Ok(())
    }
}
