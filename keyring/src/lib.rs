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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry(String);

impl Deref for Entry {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<String> for Entry {
    fn from(entry: String) -> Self {
        Self(entry)
    }
}

impl From<&String> for Entry {
    fn from(entry: &String) -> Self {
        Self(entry.clone())
    }
}

impl From<&str> for Entry {
    fn from(entry: &str) -> Self {
        Self(entry.to_owned())
    }
}

impl ToString for Entry {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Entry {
    fn get_entry(&self) -> Result<keyring::Entry> {
        keyring::Entry::new(KEYRING_SERVICE, &self.0)
            .map_err(|err| Error::GetEntryError(err, self.0.clone()))
    }

    pub fn get(&self) -> Result<String> {
        self.get_entry()?
            .get_password()
            .map_err(|err| Error::GetSecretError(err, self.0.clone()))
    }

    pub fn set<S>(&self, secret: S) -> Result<()>
    where
        S: AsRef<str>,
    {
        self.get_entry()?
            .set_password(secret.as_ref())
            .map_err(|err| Error::SetSecretError(err, self.0.clone()))
    }

    pub fn delete(&self) -> Result<()> {
        self.get_entry()?
            .delete_password()
            .map_err(|err| Error::DeleteSecretError(err, self.0.clone()))
    }
}
