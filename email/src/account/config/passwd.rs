//! Module dedicated to password configuration.
//!
//! This module contains everything related to password configuration.

use log::debug;
use secret::Secret;
use std::{
    io,
    ops::{Deref, DerefMut},
};

#[doc(inline)]
pub use super::{Error, Result};

/// The password configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct PasswdConfig(
    #[cfg_attr(
        feature = "derive",
        serde(skip_serializing_if = "Secret::is_undefined")
    )]
    pub Secret,
);

impl Deref for PasswdConfig {
    type Target = Secret;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PasswdConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PasswdConfig {
    /// If the current password secret is a keyring entry, delete it.
    pub async fn reset(&self) -> Result<()> {
        self.delete_only_keyring()
            .await
            .map_err(Error::DeletePasswordFromKeyringError)?;
        Ok(())
    }

    /// Define the password only if it does not exist in the keyring.
    pub async fn configure(&self, get_passwd: impl Fn() -> io::Result<String>) -> Result<()> {
        match self.find().await {
            Ok(None) => {
                debug!("cannot find imap password from keyring, setting it");
                let passwd = get_passwd().map_err(Error::GetFromUserError)?;
                self.set_only_keyring(passwd)
                    .await
                    .map_err(Error::SetIntoKeyringError)?;
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(err) => Err(Error::GetFromKeyringError(err).into()),
        }
    }
}
