use keyring::Entry;
use std::result;
use thiserror::Error;

use crate::{process, Cmd};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get sensitive string from command")]
    GetStringFromCmd(#[source] process::Error),
    #[error("cannot get keyring entry {1}")]
    GetStringFromKeyring(#[source] keyring::Error, String),
    #[error("cannot save keyring entry {1}")]
    SaveStringIntoKeyring(#[source] keyring::Error, String),
    #[error("cannot create keyring entry instance")]
    CreateKeyringEntryError(#[source] keyring::Error),
}

pub type Result<T> = result::Result<T, Error>;
pub type EntryName = String;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SensitiveString {
    Raw(String),
    Cmd(Cmd),
    Keyring(EntryName),
}

impl SensitiveString {
    const KEYRING_SERVICE: &str = "pimalaya";

    fn get_keyring_entry<E: AsRef<str>>(entry: E) -> Result<Entry> {
        Entry::new(Self::KEYRING_SERVICE, entry.as_ref()).map_err(Error::CreateKeyringEntryError)
    }

    pub fn new_raw<R: ToString>(raw: R) -> Self {
        Self::Raw(raw.to_string())
    }

    pub fn new_cmd<C: Into<Cmd>>(cmd: C) -> Self {
        Self::Cmd(cmd.into())
    }

    pub fn new_keyring<E: ToString>(entry: E) -> Self {
        Self::Keyring(entry.to_string())
    }

    pub fn get(&self) -> Result<String> {
        match self {
            Self::Raw(raw) => Ok(raw.clone()),
            Self::Cmd(cmd) => {
                let output = cmd.run().map_err(Error::GetStringFromCmd)?;
                Ok(String::from_utf8_lossy(&output).to_string())
            }
            Self::Keyring(entry) => Self::get_keyring_entry(entry)?
                .get_password()
                .map_err(|err| Error::GetStringFromKeyring(err, entry.clone())),
        }
    }

    pub fn set<S: ToString>(&self, sensitive_string: S) -> Result<String> {
        let sensitive_string = sensitive_string.to_string();

        if let Self::Keyring(entry) = self {
            Self::get_keyring_entry(&entry)?
                .set_password(&sensitive_string)
                .map_err(|err| Error::SaveStringIntoKeyring(err, entry.clone()))?
        }

        Ok(sensitive_string)
    }
}
