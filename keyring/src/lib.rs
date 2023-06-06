use log::debug;
use std::{ops::Deref, result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get keyring entry {1}")]
    GetEntryError(#[source] keyring::Error, String),
    #[error("cannot get keyring secret at {1}")]
    GetSecretError(#[source] keyring::Error, String),
    #[error("cannot set keyring secret at {1}")]
    SetSecretError(#[source] keyring::Error, String),
    #[error("cannot delete keyring secret at {1}")]
    DeleteSecretError(#[source] keyring::Error, String),
}

pub type Result<T> = result::Result<T, Error>;

const KEYRING_SERVICE: &str = "pimalaya";

/// Alias for the keyring entry key.
pub type Key = String;

/// Wrapper around [`keyring::Entry`] that holds a keyring entry key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry(Key);

impl Entry {
    /// Create a new keyring [`Entry`] based on the given key.
    pub fn new(key: impl ToString) -> Self {
        Self(key.to_string())
    }

    fn get_entry(&self) -> Result<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, &self)
            .map_err(|err| Error::GetEntryError(err, self.0.clone()))
    }

    /// Get the secret from the user's global keyring.
    pub fn get(&self) -> Result<String> {
        debug!("getting keyring secret for entry {:?}", self.0);
        self.get_entry()?
            .get_password()
            .map_err(|err| Error::GetSecretError(err, self.0.clone()))
    }

    /// (Re)set the secret from the user's global keyring.
    pub fn set(&self, secret: impl AsRef<str>) -> Result<()> {
        debug!("setting keyring secret for entry {:?}", self.0);
        self.get_entry()?
            .set_password(secret.as_ref())
            .map_err(|err| Error::SetSecretError(err, self.0.clone()))
    }

    /// Delete the secret from the user's global keyring.
    pub fn delete(&self) -> Result<()> {
        debug!("deleting keyring secret for entry {:?}", self.0);
        self.get_entry()?
            .delete_password()
            .map_err(|err| Error::DeleteSecretError(err, self.0.clone()))
    }
}

impl Deref for Entry {
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Entry {
    fn from(key: String) -> Self {
        Self::new(key)
    }
}

impl From<&String> for Entry {
    fn from(key: &String) -> Self {
        Self::new(key)
    }
}

impl From<&str> for Entry {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

impl ToString for Entry {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}
