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

impl Default for Secret {
    fn default() -> Self {
        Self::new_keyring("")
    }
}

impl Secret {
    pub fn new_raw<R>(raw: R) -> Self
    where
        R: ToString,
    {
        Self::Raw(raw.to_string())
    }

    pub fn new_cmd<C>(cmd: C) -> Self
    where
        C: Into<Cmd>,
    {
        Self::Cmd(cmd.into())
    }

    pub fn new_keyring<E>(entry: E) -> Self
    where
        E: Into<Entry>,
    {
        Self::Keyring(entry.into())
    }

    pub fn is_undefined_entry(&self) -> bool {
        *self == Self::default()
    }

    pub fn replace_undefined_entry_with<E>(&mut self, entry: E)
    where
        E: Into<Entry>,
    {
        if self.is_undefined_entry() {
            *self = Self::new_keyring(entry)
        }
    }

    pub fn get(&self) -> Result<String> {
        match self {
            Self::Raw(raw) => Ok(raw.clone()),
            Self::Cmd(cmd) => {
                let output = cmd.run().map_err(Error::GetSecretFromCmd)?;
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            Self::Keyring(entry) => Ok(entry.get()?),
        }
    }

    pub fn set<S>(&self, secret: S) -> Result<String>
    where
        S: AsRef<str>,
    {
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
