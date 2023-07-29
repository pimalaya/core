//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.

use log::{debug, warn};
use pimalaya_email_tpl::{
    PgpPublicKey, PgpPublicKeyResolver, PgpPublicKeys, PgpPublicKeysResolver, PgpSecretKey,
    PgpSecretKeyResolver,
};
use pimalaya_keyring::Entry;
use std::{io, path::PathBuf};
use thiserror::Error;
use tokio::fs;

use crate::Result;

/// Errors related to PGP configuration.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot delete pgp key from keyring")]
    DeletePgpKeyFromKeyringError(#[source] pimalaya_keyring::Error),
    #[error("cannot delete pgp key at {1}")]
    DeletePgpKeyAtPathError(#[source] io::Error, PathBuf),
    #[error("cannot generate pgp key pair for {1}")]
    GeneratePgpKeyPairError(#[source] pimalaya_pgp::Error, String),
    #[error("cannot export secret key to armored string")]
    ExportSecretKeyToArmoredStringError(#[source] pimalaya_pgp::NativeError),
    #[error("cannot export public key to armored string")]
    ExportPublicKeyToArmoredStringError(#[source] pimalaya_pgp::NativeError),
    #[error("cannot write secret key file at {1}")]
    WriteSecretKeyFileError(#[source] io::Error, PathBuf),
    #[error("cannot write public key file at {1}")]
    WritePublicKeyFileError(#[source] io::Error, PathBuf),
    #[error("cannot set secret key to keyring")]
    SetSecretKeyToKeyringError(#[source] pimalaya_keyring::Error),
    #[error("cannot set public key to keyring")]
    SetPublicKeyToKeyringError(#[source] pimalaya_keyring::Error),
}

/// The PGP key enum.
///
/// Determines how the user's PGP key should be retrieved: from a file
/// or from the user's global keyring system.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpKey {
    #[default]
    None,

    /// The PGP key is located at the given path.
    Path(PathBuf),

    /// The PGP key is located at the given entry of the user's global
    /// keyring system.
    Keyring(Entry),
}

impl PgpKey {
    /// Reset PGP key by deleting it from path or from the keyring.
    pub async fn reset(&self) -> Result<()> {
        match self {
            Self::None => (),
            Self::Path(path) => {
                if let Some(path) = path.as_path().to_str() {
                    let path_str = match shellexpand::full(path) {
                        Ok(path) => path.to_string(),
                        Err(err) => {
                            warn!("cannot shell expand pgp key path {path}: {err}");
                            debug!("cannot shell expand pgp key path {path:?}: {err:?}");
                            path.to_owned()
                        }
                    };

                    let path = PathBuf::from(&path_str);

                    if path.is_file() {
                        fs::remove_file(&path)
                            .await
                            .map_err(|err| Error::DeletePgpKeyAtPathError(err, path.clone()))?;
                    } else {
                        warn!("cannot delete pgp key file at {path_str}: file not found");
                    }
                } else {
                    warn!("cannot get pgp key file path as str: {path:?}");
                }
            }
            Self::Keyring(entry) => entry
                .delete_secret()
                .map_err(Error::DeletePgpKeyFromKeyringError)?,
        };

        Ok(())
    }
}

/// The PGP configuration.
// TODO: `Gpg` variant using `libgpgme`
// TODO: `Autocrypt` variant based on `pimalaya-pgp`
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpConfig {
    /// No configuration.
    #[default]
    None,

    /// Native configuration.
    Native(PgpNativeConfig),
}

impl Into<PgpSecretKey> for PgpConfig {
    fn into(self) -> PgpSecretKey {
        match self {
            Self::None => PgpSecretKey::Disabled,
            Self::Native(config) => config.into(),
        }
    }
}

impl Into<PgpPublicKey> for PgpConfig {
    fn into(self) -> PgpPublicKey {
        match self {
            Self::None => PgpPublicKey::Disabled,
            Self::Native(config) => config.into(),
        }
    }
}

impl Into<PgpPublicKeys> for PgpConfig {
    fn into(self) -> PgpPublicKeys {
        match self {
            Self::None => PgpPublicKeys::Disabled,
            Self::Native(config) => config.into(),
        }
    }
}

impl PgpConfig {
    pub async fn reset(&self) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Native(config) => config.reset().await,
        }
    }

    pub async fn configure(&self, email: impl ToString) -> Result<()> {
        match self {
            Self::None => Ok(()),
            Self::Native(config) => config.configure(email).await,
        }
    }
}

/// The native PGP configuration.
///
/// This configuration is based on the [`pgp`] crate, which provides a
/// native Rust implementation of the PGP standard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgpNativeConfig {
    pub secret_key: PgpKey,
    pub public_key: PgpKey,
    pub key_servers: Vec<String>,
}

impl PgpNativeConfig {
    pub fn default_key_servers() -> Vec<String> {
        vec![
            String::from("keys.openpgp.org"),
            String::from("keys.mailvelope.com"),
        ]
    }

    /// Deletes secret and public keys.
    pub async fn reset(&self) -> Result<()> {
        self.secret_key.reset().await?;
        self.public_key.reset().await?;
        Ok(())
    }

    /// Generates secret and public keys then stores them.
    pub async fn configure(&self, email: impl ToString) -> Result<()> {
        let email = email.to_string();

        let (skey, pkey) = pimalaya_pgp::generate_key_pair(email.clone())
            .await
            .map_err(|err| Error::GeneratePgpKeyPairError(err, email.clone()))?;
        let skey = skey
            .to_armored_string(None)
            .map_err(Error::ExportSecretKeyToArmoredStringError)?;
        let pkey = pkey
            .to_armored_string(None)
            .map_err(Error::ExportPublicKeyToArmoredStringError)?;

        match &self.secret_key {
            PgpKey::None => Entry::from(format!("pgp-secret-key-{email}"))
                .set_secret(skey)
                .map_err(Error::SetSecretKeyToKeyringError)?,
            PgpKey::Path(path) => {
                if let Some(path) = path.as_path().to_str() {
                    let path = match shellexpand::full(path) {
                        Ok(path) => PathBuf::from(path.to_string()),
                        Err(err) => {
                            warn!("cannot shell expand pgp secret key {path}: {err}");
                            debug!("cannot shell expand pgp secret key {path:?}: {err:?}");
                            PathBuf::from(path)
                        }
                    };
                    fs::write(&path, skey)
                        .await
                        .map_err(|err| Error::WriteSecretKeyFileError(err, path))?;
                } else {
                    warn!("cannot get pgp secret key path as str: {path:?}");
                }
            }
            PgpKey::Keyring(entry) => entry
                .set_secret(skey)
                .map_err(Error::SetSecretKeyToKeyringError)?,
        }

        match &self.public_key {
            PgpKey::None => Entry::from(format!("pgp-public-key-{email}"))
                .set_secret(pkey)
                .map_err(Error::SetPublicKeyToKeyringError)?,
            PgpKey::Path(path) => {
                if let Some(path) = path.as_path().to_str() {
                    let path = match shellexpand::full(path) {
                        Ok(path) => PathBuf::from(path.to_string()),
                        Err(err) => {
                            warn!("cannot shell expand pgp public key {path}: {err}");
                            debug!("cannot shell expand pgp public key path {path:?}: {err:?}");
                            PathBuf::from(path)
                        }
                    };
                    fs::write(&path, pkey)
                        .await
                        .map_err(|err| Error::WritePublicKeyFileError(err, path))?;
                } else {
                    warn!("cannot get pgp public key path as str: {path:?}");
                }
            }
            PgpKey::Keyring(entry) => entry
                .set_secret(pkey)
                .map_err(Error::SetPublicKeyToKeyringError)?,
        }

        Ok(())
    }
}

impl Default for PgpNativeConfig {
    fn default() -> Self {
        Self {
            secret_key: Default::default(),
            public_key: Default::default(),
            key_servers: Self::default_key_servers(),
        }
    }
}

impl Into<PgpSecretKey> for PgpNativeConfig {
    fn into(self) -> PgpSecretKey {
        match self.secret_key {
            PgpKey::None => PgpSecretKey::Disabled,
            PgpKey::Path(path) => PgpSecretKey::Enabled(vec![PgpSecretKeyResolver::Path(path)]),
            PgpKey::Keyring(entry) => {
                PgpSecretKey::Enabled(vec![PgpSecretKeyResolver::Keyring(entry)])
            }
        }
    }
}

impl Into<PgpPublicKeys> for PgpNativeConfig {
    fn into(self) -> PgpPublicKeys {
        PgpPublicKeys::Enabled(vec![
            PgpPublicKeysResolver::Wkd,
            PgpPublicKeysResolver::KeyServers(self.key_servers),
        ])
    }
}

impl Into<PgpPublicKey> for PgpNativeConfig {
    fn into(self) -> PgpPublicKey {
        PgpPublicKey::Enabled(vec![
            PgpPublicKeyResolver::Wkd,
            PgpPublicKeyResolver::KeyServers(self.key_servers),
        ])
    }
}
