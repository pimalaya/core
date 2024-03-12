//! # Keyutils cache entry
//!
//! Module dedicated to keyutils entry management. The keyutils entry
//! is based on keyutils, a safe in-memory keyring that comes with
//! recent linux kernels. Since it is in-memory only, data does not
//! persist. So it is used as a cache system.

use keyring_native::keyutils::KeyutilsCredential;
use log::debug;
use std::result;
use thiserror::Error;
use tokio::task::{self, JoinError};

use crate::get_global_service_name;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build keyutils credentials using key {1}")]
    BuildCredentialsError(#[source] keyring_native::Error, String),
    #[error("cannot find secret from keyutils matching `{1}`")]
    FindSecretError(#[source] keyring_native::Error, String),
    #[error("cannot set secret from keyutils matching `{1}`")]
    SetSecretError(#[source] keyring_native::Error, String),
    #[error("cannot delete secret from keyutils matching `{1}`")]
    DeleteSecretError(#[source] keyring_native::Error, String),

    #[error(transparent)]
    JoinError(#[from] JoinError),
}

/// Result alias dedicated to keyutils entry.
pub type Result<T> = result::Result<T, Error>;

/// Keyutils entry structure.
///
/// This structure represents an entry in the keyutils keyring.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheEntry(String);

impl CacheEntry {
    /// Create a new keyutils entry with a key.
    pub fn new(key: impl ToString) -> Self {
        Self(key.to_string())
    }

    /// Get the inner key of the keyutils entry.
    pub fn get_key(&self) -> &str {
        self.0.as_ref()
    }

    /// Take the inner key of the keyutils entry.
    pub fn take_key(self) -> String {
        self.0
    }

    /// Create a new native keyutils entry instance.
    fn new_native_entry(&self) -> Result<keyring_native::Entry> {
        // a service name is always present, so unwrap() is safe here
        let service = get_global_service_name();
        let creds = KeyutilsCredential::new_with_target(None, service, service)
            .map_err(|err| Error::BuildCredentialsError(err, self.clone().take_key()))?;
        let entry = keyring_native::Entry::new_with_credential(Box::new(creds));

        Ok(entry)
    }

    /// Find the secret of the keyutils entry.
    ///
    /// Returns `None` in case the secret is not found.
    pub async fn find_secret(&self) -> Result<Option<String>> {
        debug!("finding keyutils secret for key `{}`", self.get_key());

        let entry = self.new_native_entry()?;
        let secret = task::spawn_blocking(move || entry.get_password()).await?;

        match secret {
            Err(keyring_native::Error::NoEntry) => Ok(None),
            Err(err) => Err(Error::FindSecretError(err, self.clone().take_key())),
            Ok(secret) => Ok(Some(secret)),
        }
    }

    /// (Re)set the secret of the keyutils entry.
    pub async fn set_secret(&self, secret: impl ToString) -> Result<()> {
        debug!("setting keyutils secret for key `{}`", self.get_key());

        let entry = self.new_native_entry()?;
        let secret = secret.to_string();

        task::spawn_blocking(move || entry.set_password(&secret))
            .await?
            .map_err(|err| Error::SetSecretError(err, self.clone().take_key()))
    }

    /// Delete the secret of the keyutils entry.
    pub async fn delete_secret(&self) -> Result<()> {
        debug!("deleting keyring secret for key `{}`", self.get_key());

        let entry = self.new_native_entry()?;

        task::spawn_blocking(move || entry.delete_password())
            .await?
            .map_err(|err| Error::DeleteSecretError(err, self.clone().take_key()))
    }
}

impl<T: ToString> From<T> for CacheEntry {
    fn from(key: T) -> Self {
        Self::new(key)
    }
}

impl From<CacheEntry> for String {
    fn from(val: CacheEntry) -> Self {
        val.take_key()
    }
}
