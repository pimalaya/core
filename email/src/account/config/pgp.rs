//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.

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
            Self::Path(path) => fs::remove_file(path)
                .await
                .map_err(|err| Error::DeletePgpKeyAtPathError(err, path.clone()))?,
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

/// The native PGP configuration.
///
/// This configuration is based on the [`pgp`] crate, which provides a
/// native Rust implementation of the PGP standard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PgpNativeConfig {
    secret_key: PgpKey,
    public_key: PgpKey,
    key_servers: Vec<String>,
}

impl PgpNativeConfig {
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
            PgpKey::Keyring(entry) => entry
                .set_secret(skey)
                .map_err(Error::SetSecretKeyToKeyringError)?,
            PgpKey::Path(path) => fs::write(path, skey)
                .await
                .map_err(|err| Error::WriteSecretKeyFileError(err, path.clone()))?,
        }

        match &self.public_key {
            PgpKey::None => Entry::from(format!("pgp-public-key-{email}"))
                .set_secret(pkey)
                .map_err(Error::SetPublicKeyToKeyringError)?,
            PgpKey::Keyring(entry) => entry
                .set_secret(pkey)
                .map_err(Error::SetPublicKeyToKeyringError)?,
            PgpKey::Path(path) => fs::write(path, pkey)
                .await
                .map_err(|err| Error::WritePublicKeyFileError(err, path.clone()))?,
        }

        Ok(())
    }
}

impl Default for PgpNativeConfig {
    fn default() -> Self {
        Self {
            secret_key: Default::default(),
            public_key: Default::default(),
            key_servers: vec![
                String::from("keys.openpgp.org"),
                String::from("keys.mailvelope.com"),
            ],
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
