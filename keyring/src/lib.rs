#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

mod error;
mod service;

use std::sync::Arc;

pub use keyring_native as native;
use tracing::debug;

#[doc(inline)]
pub use crate::{
    error::{Error, Result},
    service::{get_global_service_name, set_global_service_name},
};

#[cfg(any(
    all(feature = "tokio", feature = "async-std"),
    not(any(feature = "tokio", feature = "async-std"))
))]
compile_error!("Either feature `tokio` or `async-std` must be enabled for this crate.");

#[cfg(any(
    all(feature = "rustls", feature = "openssl"),
    not(any(feature = "rustls", feature = "openssl"))
))]
compile_error!("Either feature `rustls` or `openssl` must be enabled for this crate.");

/// The representation of a keyring entry.
///
/// This struct is a simple wrapper around [`native::Entry`] that
/// holds a keyring entry key.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "String", into = "String")
)]
pub struct KeyringEntry {
    /// The key used to identify the current keyring entry.
    pub key: String,

    /// The native keyring entry.
    entry: Arc<native::Entry>,
}

impl Eq for KeyringEntry {}

impl PartialEq for KeyringEntry {
    /// Two keyring entries are considered equal if their key are
    /// equal.
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl KeyringEntry {
    /// Creates a new keyring entry from a key.
    pub fn try_new(key: impl ToString) -> Result<Self> {
        Self::try_from(key.to_string())
    }

    /// Gets the secret of the keyring entry.
    pub async fn get_secret(&self) -> Result<String> {
        let key = &self.key;
        debug!(key, "get keyring secret");

        let entry = self.entry.clone();
        let secret = spawn_blocking(move || entry.get_password())
            .await?
            .map_err(|err| Error::GetSecretError(err, key.clone()))?;

        Ok(secret)
    }

    /// Finds the secret of the keyring entry.
    ///
    /// This function is like [`KeyringEntry::get_secret`], except
    /// that it returns `None` in case the secret cannot be found.
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

    /// (Re)sets the secret of the keyring entry.
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

    /// (Re)sets the secret of the keyring entry, using the builder
    /// pattern.
    ///
    /// This function acts like [`KeyringEntry::set_secret`], except
    /// that it returns [`Self`] instead of `()`.
    pub async fn try_with_secret(self, secret: impl ToString) -> Result<Self> {
        self.set_secret(secret).await?;
        Ok(self)
    }

    /// Deletes the secret of the keyring entry.
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

    /// Creates a new keyring entry from a `String`.
    ///
    /// This implementation is a wrapper around
    /// [`native::Entry::new`], where the service name is taken
    /// globally from [`get_global_service_name`].
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
    /// Returns the key of the current keyring entry.
    fn from(entry: KeyringEntry) -> Self {
        entry.key
    }
}

/// Spawns a blocking task using [`async_std`].
#[cfg(feature = "async-std")]
async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(async_std::task::spawn_blocking(f).await)
}

/// Spawns a blocking task using [`tokio`].
#[cfg(feature = "tokio")]
async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(tokio::task::spawn_blocking(f).await?)
}
