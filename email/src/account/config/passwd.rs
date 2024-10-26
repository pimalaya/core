//! Module dedicated to password configuration.
//!
//! This module contains everything related to password configuration.

use std::{
    io,
    ops::{Deref, DerefMut},
};

use secret::Secret;

#[doc(inline)]
pub use super::{Error, Result};

/// The password configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(transparent)
)]
pub struct PasswordConfig(
    #[cfg_attr(feature = "derive", serde(skip_serializing_if = "Secret::is_empty"))] pub Secret,
);

impl Deref for PasswordConfig {
    type Target = Secret;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PasswordConfig {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl PasswordConfig {
    /// If the current password secret is a keyring entry, delete it.
    pub async fn reset(&self) -> Result<()> {
        #[cfg(feature = "keyring")]
        self.delete_if_keyring()
            .await
            .map_err(Error::DeletePasswordFromKeyringError)?;

        Ok(())
    }

    /// Define the password only if it does not exist in the keyring.
    pub async fn configure<F>(
        &self,
        #[cfg_attr(not(feature = "keyring"), allow(unused_variables))] get_passwd: F,
    ) -> Result<()>
    where
        F: Fn() -> io::Result<String>,
    {
        match self.find().await {
            #[cfg(feature = "keyring")]
            Ok(None) => {
                tracing::debug!("cannot find imap password from keyring, setting it");

                let passwd = get_passwd().map_err(Error::GetFromUserError)?;

                self.set_if_keyring(passwd)
                    .await
                    .map_err(Error::SetIntoKeyringError)?;

                Ok(())
            }
            Ok(_) => Ok(()),
            Err(err) => Err(Error::GetFromKeyringError(err)),
        }
    }
}
