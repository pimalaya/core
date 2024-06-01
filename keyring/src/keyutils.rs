//! # Keyutils cache entry
//!
//! Module dedicated to keyutils entry management. The keyutils entry
//! is based on `keyutils`, a safe in-memory keyring that comes with
//! recent linux kernels. Since it is in-memory only, data does not
//! persist. So it is used as a cache system.

use keyring_native::keyutils::KeyutilsCredential;
use log::debug;
use std::sync::Arc;
use tokio::task;

use crate::{get_global_service_name, Error, Result};

/// Keyutils cache entry structure.
///
/// This structure represents the cache entry in the linux keyutils
/// keyring.
#[derive(Debug, Clone)]
pub struct KeyutilsEntry {
    /// The keyutils cache entry key.
    pub key: String,

    /// The atomic reference to the native keyutils entry.
    entry: Arc<keyring_native::Entry>,
}

impl KeyutilsEntry {
    /// Create a new keyutils entry with a key.
    pub fn try_new(key: impl ToString) -> Result<Self> {
        let service = get_global_service_name();
        let key = key.to_string();
        let creds = KeyutilsCredential::new_with_target(Some(&key), service, service)
            .map_err(|err| Error::BuildCredentialsError(err, key.clone()))?;
        let entry = keyring_native::Entry::new_with_credential(Box::new(creds));
        let entry = Arc::new(entry);

        Ok(Self { key, entry })
    }

    /// Find the secret of the keyutils entry.
    ///
    /// Returns `None` in case the secret is not found.
    pub async fn find_secret(&self) -> Result<Option<String>> {
        debug!("finding keyutils secret for key `{}`", self.key);

        let entry = self.entry.clone();
        let secret = task::spawn_blocking(move || entry.get_password()).await?;

        match secret {
            Err(keyring_native::Error::NoEntry) => Ok(None),
            Err(err) => Err(Error::FindSecretError(err, self.key.clone())),
            Ok(secret) => Ok(Some(secret)),
        }
    }

    /// (Re)set the secret of the keyutils entry.
    pub async fn set_secret(&self, secret: impl ToString) -> Result<()> {
        debug!("setting keyutils secret for key `{}`", self.key);

        let entry = self.entry.clone();
        let secret = secret.to_string();

        task::spawn_blocking(move || entry.set_password(&secret))
            .await?
            .map_err(|err| Error::SetSecretError(err, self.key.clone()))
    }

    /// Delete the secret of the keyutils entry.
    pub async fn delete_secret(&self) -> Result<()> {
        debug!("deleting keyring secret for key `{}`", self.key);

        let entry = self.entry.clone();

        task::spawn_blocking(move || entry.delete_password())
            .await?
            .map_err(|err| Error::DeleteSecretError(err, self.key.clone()))
    }
}
