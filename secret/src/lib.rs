use pimalaya_keyring::Entry;
use pimalaya_process::Cmd;
use std::result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get secret from command")]
    GetSecretFromCmd(#[source] pimalaya_process::Error),
    #[error(transparent)]
    KeyringError(#[from] pimalaya_keyring::Error),
}

impl Error {
    pub fn is_get_secret_error(&self) -> bool {
        matches!(
            self,
            Self::KeyringError(pimalaya_keyring::Error::GetSecretError(_, _))
        )
    }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Secret {
    Raw(String),
    Cmd(Cmd),
    Keyring(Entry),
}

impl Secret {
    pub fn new_raw<R: ToString>(raw: R) -> Self {
        Self::Raw(raw.to_string())
    }

    pub fn new_cmd<C: Into<Cmd>>(cmd: C) -> Self {
        Self::Cmd(cmd.into())
    }

    pub fn new_keyring<E: Into<Entry>>(entry: E) -> Self {
        Self::Keyring(entry.into())
    }

    pub fn get(&self) -> Result<String> {
        match self {
            Self::Raw(raw) => Ok(raw.clone()),
            Self::Cmd(cmd) => {
                let output = cmd.run().map_err(Error::GetSecretFromCmd)?;
                Ok(String::from_utf8_lossy(&output).to_string())
            }
            Self::Keyring(entry) => Ok(entry.get()?),
        }
    }

    pub fn set<S: AsRef<str>>(&self, secret: S) -> Result<String> {
        if let Self::Keyring(entry) = self {
            entry.set(secret.as_ref())?;
        }

        Ok(secret.as_ref().to_string())
    }

    pub fn delete(&self) -> Result<()> {
        match self {
            Self::Raw(_) => Ok(()),
            Self::Cmd(_) => Ok(()),
            Self::Keyring(entry) => Ok(entry.delete()?),
        }
    }
}
