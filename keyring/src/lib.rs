//! # Keyring
//!
//! Manage credentials using OS-specific keyrings: Secret Service and
//! keyutils on Linux, Security Framework on MacOS and Security
//! Credentials on Windows.
//!
//! The aim of this library is to provide a convenient wrapper around
//! [keyring-rs](https://crates.io/crates/keyring), a cross-platform
//! library to manage credentials. The main structure is
//! [`KeyringEntry`].

mod error;
mod service;

pub use native;
use std::sync::Arc;
use tracing::debug;

#[doc(inline)]
pub use crate::{
    error::{Error, Result},
    service::{get_global_service_name, set_global_service_name},
};

/// The keyring entry.
///
/// This struct is a simple wrapper around [`native::Entry`] that
/// holds a keyring entry key, as well as a keyutils entry on Linux
/// for cache.
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
    entry: Arc<native::Entry>,
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
        debug!(key, "get keyring secret");

        let entry = self.entry.clone();
        let secret = spawn_blocking(move || entry.get_password())
            .await?
            .map_err(|err| Error::GetSecretError(err, key.clone()))?;

        Ok(secret)
    }

    /// Find the secret of the keyring entry.
    ///
    /// Returns `None` in case the secret is not found.
    pub async fn find_secret(&self) -> Result<Option<String>> {
        let key = &self.key;
        debug!(key, "find keyring secret");

        let entry = self.entry.clone();
        let secret = spawn_blocking(move || entry.get_password()).await?;

        match secret {
            Err(native::Error::NoEntry) => Ok(None),
            Err(err) => Err(Error::FindSecretError(err, key.clone())),
            Ok(secret) => Ok(Some(secret)),
        }
    }

    /// (Re)set the secret of the keyring entry.
    pub async fn set_secret(&self, secret: impl ToString) -> Result<()> {
        let key = &self.key;
        debug!(key, "set keyring secret");

        let secret = secret.to_string();
        let entry = self.entry.clone();
        spawn_blocking(move || entry.set_password(&secret))
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
        debug!(key, "delete keyring secret");

        let entry = self.entry.clone();
        spawn_blocking(move || entry.delete_credential())
            .await?
            .map_err(|err| Error::DeleteSecretError(err, key.clone()))?;

        Ok(())
    }
}

impl TryFrom<String> for KeyringEntry {
    type Error = Error;

    fn try_from(key: String) -> Result<Self> {
        let service = get_global_service_name();

        let entry = match native::Entry::new(service, &key) {
            Ok(entry) => Ok(Arc::new(entry)),
            Err(err) => Err(Error::BuildEntryError(err, key.clone())),
        }?;

        Ok(Self { key, entry })
    }
}

impl From<KeyringEntry> for String {
    fn from(entry: KeyringEntry) -> Self {
        entry.key
    }
}

#[cfg(feature = "async-std")]
async fn spawn_blocking<T: Send + 'static>(f: impl Fn() -> T + Send + 'static) -> Result<T> {
    Ok(async_std::task::spawn_blocking(f).await)
}

#[cfg(feature = "tokio")]
async fn spawn_blocking<T: Send + 'static>(f: impl Fn() -> T + Send + 'static) -> Result<T> {
    Ok(tokio::task::spawn_blocking(f).await?)
}
