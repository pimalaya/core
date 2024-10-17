//! # Secret
//!
//! The core concept of this library is to abstract the concept of
//! secret. A secret can be retrieved either from a raw string, from a
//! command or from a keyring entry. The associated structure is
//! [`Secret`].

#[cfg(feature = "derive")]
pub mod derive;
mod error;

#[cfg(feature = "keyring")]
pub use keyring;
#[cfg(feature = "keyring")]
use keyring::KeyringEntry;
use log::debug;
#[cfg(feature = "command")]
pub use process;
#[cfg(feature = "command")]
use process::Command;

#[doc(inline)]
pub use crate::error::{Error, Result};

/// The secret.
///
/// A secret can be retrieved either from a raw string, from a shell
/// command or from a keyring entry.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case"),
    serde(from = "derive::Secret")
)]
pub enum Secret {
    /// The secret is contained in a raw string, usually not safe to
    /// use and so not recommended.
    Raw(String),

    /// The secret is exposed by the given shell command.
    #[cfg(feature = "command")]
    #[cfg_attr(feature = "derive", serde(alias = "cmd"))]
    Command(Command),

    /// The secret is contained in the given user's global keyring at
    /// the given entry.
    #[cfg(feature = "keyring")]
    #[cfg_attr(feature = "derive", serde(rename = "keyring"))]
    KeyringEntry(KeyringEntry),

    /// The secret is not defined.
    #[default]
    #[cfg_attr(feature = "derive", serde(skip_serializing))]
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
    pub fn new_command(cmd: impl Into<Command>) -> Self {
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

    /// Get the secret value.
    ///
    /// The command-based secret execute its shell command and returns
    /// the output, and the keyring-based secret retrieves the value
    /// from the global keyring using its inner key.
    pub async fn get(&self) -> Result<String> {
        match self {
            Self::Raw(raw) => Ok(raw.clone()),
            #[cfg(feature = "command")]
            Self::Command(cmd) => Ok(cmd
                .run()
                .await
                .map_err(Error::GetSecretFromCommand)?
                .to_string_lossy()
                .lines()
                .take(1)
                .next()
                .ok_or(Error::GetSecretFromCommandEmptyOutputError)?
                .to_owned()),
            #[cfg(feature = "keyring")]
            Self::KeyringEntry(entry) => {
                Ok(entry.get_secret().await.map_err(Error::KeyringError)?)
            }
            Self::Undefined => Err(Error::GetUndefinedSecretError),
        }
    }

    /// Find the secret value.
    ///
    /// Like [`get`], but returns [`None`] if the secret value is not
    /// found or undefined.
    pub async fn find(&self) -> Result<Option<String>> {
        match self {
            Self::Raw(secret) => Ok(Some(secret.clone())),
            #[cfg(feature = "command")]
            Self::Command(cmd) => Ok(cmd
                .run()
                .await
                .map_err(Error::GetSecretFromCommand)?
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

    /// Change the secret value.
    ///
    /// This is only applicable for raw secrets and keyring-based
    /// secrets. A secret value cannot be changed for command-base
    /// secrets, since the value is the output of the command.
    pub async fn set(&mut self, secret: impl AsRef<str>) -> Result<String> {
        let secret = secret.as_ref();

        match self {
            Self::Raw(prev) => {
                *prev = secret.to_owned();
            }
            #[cfg(feature = "command")]
            Self::Command(_) => {
                debug!("cannot change value of command-based secret");
            }
            #[cfg(feature = "keyring")]
            Self::KeyringEntry(entry) => {
                entry
                    .set_secret(secret)
                    .await
                    .map_err(Error::KeyringError)?;
            }
            Self::Undefined => {
                debug!("cannot change value of undefined secret");
            }
        }

        Ok(secret.to_owned())
    }

    /// Change the secret value of the keyring-based secret only.
    ///
    /// This function as no effect on other secret variants.
    #[cfg(feature = "keyring")]
    pub async fn set_only_keyring(&self, secret: impl AsRef<str>) -> Result<String> {
        let secret = secret.as_ref();

        if let Self::KeyringEntry(entry) = self {
            entry
                .set_secret(secret)
                .await
                .map_err(Error::KeyringError)?;
        }

        Ok(secret.to_owned())
    }

    /// Replace undefined secret by a keyring-based one.
    ///
    /// This function has no effect on other variants.
    #[cfg(feature = "keyring")]
    pub fn replace_undefined_to_keyring(
        &mut self,
        entry: impl TryInto<KeyringEntry, Error = keyring::Error>,
    ) -> Result<()> {
        if self.is_undefined() {
            *self = Self::try_new_keyring_entry(entry)?
        }

        Ok(())
    }

    /// Delete the secret value and make the current secret undefined.
    pub async fn delete(&mut self) -> Result<()> {
        #[cfg(feature = "keyring")]
        if let Self::KeyringEntry(entry) = self {
            entry.delete_secret().await.map_err(Error::KeyringError)?;
        }

        *self = Self::Undefined;

        Ok(())
    }

    /// Delete the secret value of keyring-based secrets only.
    ///
    /// This function has no effect on other variants.
    #[cfg(feature = "keyring")]
    pub async fn delete_only_keyring(&self) -> Result<()> {
        if let Self::KeyringEntry(entry) = self {
            entry.delete_secret().await.map_err(Error::KeyringError)?;
        }

        Ok(())
    }
}
