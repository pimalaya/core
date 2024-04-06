//! # Keyring
//!
//! Manage credentials using OS-specific keyrings: Secret Service and
//! keyutils on Linux, Security Framework on MacOS and Security
//! Credentials on Windows.
//!
//! The aim of this library is to provide a convenient wrapper around
//! [keyring-rs](https://crates.io/crates/keyring), a cross-platform
//! library to manage credentials. The main structure is
//! [`KeyringEntry`]. Cache is enabled on Linux only, using the kernel
//! [`keyutils`] keyring.

mod error;
#[cfg(target_os = "linux")]
mod keyutils;
mod service;

pub use keyring_native as native;
use log::{debug, trace};
use std::sync::Arc;
use tokio::task;

#[cfg(target_os = "linux")]
#[doc(inline)]
pub use crate::keyutils::KeyutilsEntry;
#[doc(inline)]
pub use crate::{
    error::{Error, Result},
    service::{get_global_service_name, set_global_service_name},
};

/// The keyring entry.
///
/// This struct is a simple wrapper around [`keyring_native::Entry`]
/// that holds a keyring entry key, as well as a keyutils entry on
/// Linux for cache.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "String", into = "String")
)]
pub struct KeyringEntry {
    /// The key of the keyring entry.
    pub key: String,

    /// The native keyring entry.
    entry: Arc<keyring_native::Entry>,

    /// The cache keyutils entry.
    #[cfg(target_os = "linux")]
    cache_entry: KeyutilsEntry,
}

impl Eq for KeyringEntry {}

impl PartialEq for KeyringEntry {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl KeyringEntry {
    /// Create a new keyring entry from a key.
    pub fn try_new(key: impl ToString) -> Result<Self> {
        Self::try_from(key.to_string())
    }

    /// Get the secret of the keyring entry.
    pub async fn get_secret(&self) -> Result<String> {
        let key = &self.key;

        #[cfg(target_os = "linux")]
        match self.cache_entry.find_secret().await {
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

        let entry = self.entry.clone();
        let secret = task::spawn_blocking(move || entry.get_password())
            .await?
            .map_err(|err| Error::GetSecretError(err, key.clone()))?;

        Ok(secret)
    }

    /// Find the secret of the keyring entry.
    ///
    /// Returns `None` in case the secret is not found.
    pub async fn find_secret(&self) -> Result<Option<String>> {
        let key = &self.key;

        #[cfg(target_os = "linux")]
        match self.cache_entry.find_secret().await {
            Ok(Some(secret)) => {
                debug!("found secret from cache matching `{key}`");
                return Ok(Some(secret));
            }
            Ok(None) => {
                debug!("no secret found from cache matching `{key}`");
            }
            Err(err) => {
                debug!("cannot find secret from cache matching `{key}`");
                trace!("{err:?}");
            }
        }

        debug!("finding keyring secret for key `{key}`");

        let entry = self.entry.clone();
        let secret = task::spawn_blocking(move || entry.get_password()).await?;

        match secret {
            Err(keyring_native::Error::NoEntry) => Ok(None),
            Err(err) => Err(Error::FindSecretError(err, key.clone())),
            Ok(secret) => Ok(Some(secret)),
        }
    }

    /// (Re)set the secret of the keyring entry.
    pub async fn set_secret(&self, secret: impl ToString) -> Result<()> {
        let key = &self.key;
        let secret = secret.to_string();

        debug!("setting keyring secret for key `{key}`");

        #[cfg(target_os = "linux")]
        self.cache_entry.set_secret(&secret).await?;

        let entry = self.entry.clone();
        task::spawn_blocking(move || entry.set_password(&secret))
            .await?
            .map_err(|err| Error::SetSecretError(err, key.clone()))?;

        Ok(())
    }

    /// (Re)set the secret of the keyring entry, using the builder
    /// pattern.
    pub async fn try_with_secret(self, secret: impl ToString) -> Result<Self> {
        self.set_secret(secret).await?;
        Ok(self)
    }

    /// Delete the secret of the keyring entry.
    pub async fn delete_secret(&self) -> Result<()> {
        let key = &self.key;

        debug!("deleting keyring secret for key `{key}`");

        #[cfg(target_os = "linux")]
        self.cache_entry.delete_secret().await?;

        let entry = self.entry.clone();
        task::spawn_blocking(move || entry.delete_password())
            .await?
            .map_err(|err| Error::DeleteSecretError(err, key.clone()))?;

        Ok(())
    }
}

impl TryFrom<String> for KeyringEntry {
    type Error = Error;

    fn try_from(key: String) -> Result<Self> {
        let service = get_global_service_name();

        let entry = match keyring_native::Entry::new(service, &key) {
            Ok(entry) => Ok(Arc::new(entry)),
            Err(err) => Err(Error::BuildEntryError(err, key.clone())),
        }?;

        let cache_entry = KeyutilsEntry::try_new(&key)?;

        Ok(Self {
            key,
            entry,
            cache_entry,
        })
    }
}

impl From<KeyringEntry> for String {
    fn from(entry: KeyringEntry) -> Self {
        entry.key
    }
}
