//! # Keyring
//!
//! TODO

#[cfg(target_os = "linux")]
mod cache;
mod service;

use log::{debug, trace};
use serde::{Deserialize, Serialize};
use std::result;
use thiserror::Error;
use tokio::task::{self, JoinError};

#[cfg(target_os = "linux")]
#[doc(inline)]
pub use cache::CacheEntry;
#[doc(inline)]
pub use service::{get_global_service_name, set_global_service_name};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build keyring entry using key `{1}`")]
    BuildEntryError(#[source] keyring_native::Error, String),
    #[error("cannot get secret from keyring matching `{1}`")]
    GetSecretError(#[source] keyring_native::Error, String),
    #[error("cannot find secret from keyring matching `{1}`")]
    FindSecretError(#[source] keyring_native::Error, String),
    #[error("cannot set secret from keyring matching `{1}`")]
    SetSecretError(#[source] keyring_native::Error, String),
    #[error("cannot delete secret from keyring matching `{1}`")]
    DeleteSecretError(#[source] keyring_native::Error, String),

    #[cfg(target_os = "linux")]
    #[error("error while using keyutils cache")]
    KeyutilsCacheError(#[source] cache::Error),

    #[error(transparent)]
    JoinError(#[from] JoinError),
}

/// Result alias dedicated to keyring entry.
pub type Result<T> = result::Result<T, Error>;

/// Alias for the keyring entry key.
pub type Key = String;

/// Keyring entry wrapper.
///
/// This struct is a simple wrapper around [`native::Entry`] that
/// holds a keyring entry key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Entry(Key);

impl Entry {
    /// Create a new keyring entry with an entry key.
    pub fn new(key: impl ToString) -> Self {
        Self(key.to_string())
    }

    /// Get the inner key of the keyring entry.
    pub fn get_key(&self) -> &str {
        self.0.as_ref()
    }

    /// Take the inner key of the keyring entry.
    pub fn take_key(self) -> String {
        self.0
    }

    /// Create a new native keyring entry instance.
    fn new_native_entry(&self) -> Result<keyring_native::Entry> {
        // a service name is always present, so unwrap() is safe here
        let service = get_global_service_name();
        keyring_native::Entry::new(service, self.get_key())
            .map_err(|err| Error::BuildEntryError(err, self.clone().take_key()))
    }

    /// Create a cache entry from the current entry.
    #[cfg(target_os = "linux")]
    fn to_cache_entry(&self) -> CacheEntry {
        CacheEntry::new(self.get_key().to_owned())
    }

    /// Get the secret of the keyring entry.
    pub async fn get_secret(&self) -> Result<String> {
        let key = self.get_key();

        #[cfg(target_os = "linux")]
        match self.to_cache_entry().find_secret().await {
            Ok(Some(secret)) => {
                debug!("found secret from cache matching `{key}`");
                return Ok(secret);
            }
            Ok(None) => {
                debug!("no secret found from cache matching `{key}`");
            }
            Err(err) => {
                debug!("cannot find secret from cache matching `{key}`");
                trace!("{err:?}");
            }
        }

        debug!("getting keyring secret for key `{key}`");

        let entry = self.new_native_entry()?;

        task::spawn_blocking(move || entry.get_password())
            .await?
            .map_err(|err| Error::GetSecretError(err, key.to_owned()))
    }

    /// Find the secret of the keyring entry.
    ///
    /// Returns `None` in case the secret is not found.
    pub async fn find_secret(&self) -> Result<Option<String>> {
        #[cfg(target_os = "linux")]
        match self.to_cache_entry().find_secret().await {
            Ok(Some(secret)) => {
                debug!("found secret from cache matching `{}`", self.get_key());
                return Ok(Some(secret));
            }
            Ok(None) => {
                debug!("no secret found from cache matching `{}`", self.get_key());
            }
            Err(err) => {
                debug!(
                    "cannot find secret from cache matching `{}`",
                    self.get_key()
                );
                trace!("{err:?}");
            }
        }

        debug!("finding keyring secret for key `{}`", self.get_key());

        let entry = self.new_native_entry()?;
        let secret = task::spawn_blocking(move || entry.get_password()).await?;

        match secret {
            Err(keyring_native::Error::NoEntry) => Ok(None),
            Err(err) => Err(Error::FindSecretError(err, self.clone().take_key())),
            Ok(secret) => Ok(Some(secret)),
        }
    }

    /// (Re)set the secret of the keyring entry.
    pub async fn set_secret(&self, secret: impl ToString) -> Result<()> {
        debug!("setting keyring secret for key `{}`", self.get_key());

        let entry = self.new_native_entry()?;
        let secret = secret.to_string();

        #[cfg(target_os = "linux")]
        self.to_cache_entry()
            .set_secret(&secret)
            .await
            .map_err(Error::KeyutilsCacheError)?;

        task::spawn_blocking(move || entry.set_password(&secret))
            .await?
            .map_err(|err| Error::SetSecretError(err, self.clone().take_key()))
    }

    /// Delete the secret of the keyring entry.
    pub async fn delete_secret(&self) -> Result<()> {
        debug!("deleting keyring secret for key `{}`", self.get_key());

        let entry = self.new_native_entry()?;

        #[cfg(target_os = "linux")]
        self.to_cache_entry()
            .delete_secret()
            .await
            .map_err(Error::KeyutilsCacheError)?;

        task::spawn_blocking(move || entry.delete_password())
            .await?
            .map_err(|err| Error::DeleteSecretError(err, self.clone().take_key()))
    }
}

impl<T: ToString> From<T> for Entry {
    fn from(key: T) -> Self {
        Self::new(key)
    }
}

impl From<Entry> for String {
    fn from(val: Entry) -> Self {
        val.take_key()
    }
}
