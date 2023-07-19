//! Module dedicated to PGP configuration.
//!
//! This module contains everything related to PGP configuration.

use pimalaya_keyring::Entry;
use pimalaya_process::Cmd;
use thiserror::Error;

use crate::{account, Result};

/// Errors related to PGP configuration.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot delete secret key from keyring")]
    DeleteSecretKeyFromKeyringError(#[source] pimalaya_keyring::Error),
    #[error("cannot delete public key from keyring")]
    DeletePublicKeyFromKeyringError(#[source] pimalaya_keyring::Error),
    #[error("cannot export secret key to armored string")]
    ExportSecretKeyToArmoredStringError(#[source] pgp::errors::Error),
    #[error("cannot set secret key to keyring")]
    SetSecretKeyToKeyringError(#[source] pimalaya_keyring::Error),
    #[error("cannot export public key to armored string")]
    ExportPublicKeyToArmoredStringError(#[source] pgp::errors::Error),
    #[error("cannot set public key to keyring")]
    SetPublicKeyToKeyringError(#[source] pimalaya_keyring::Error),
}

/// The PGP configuration.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpConfig {
    #[default]
    None,

    /// Native configuration.
    Native(PgpNativeConfig),

    /// GPG configuration.
    Gpg(PgpGpgConfig),

    /// Commands configuration.
    Cmd(PgpCmdConfig),
}

/// The native PGP configuration.
///
/// This configuration is based on the [`pgp`] crate, which provides a
/// native Rust implementation of the PGP standard.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpNativeConfig {
    //
}

impl PgpNativeConfig {
    fn get_secret_key_entry(email: impl AsRef<str>) -> Entry {
        (String::from("pgp-secret-key-") + email.as_ref()).into()
    }

    fn get_public_key_entry(email: impl AsRef<str>) -> Entry {
        (String::from("pgp-public-key-") + email.as_ref()).into()
    }

    /// Deletes secret and public keys from the global keyring.
    pub fn reset(&self, email: impl AsRef<str>) -> Result<()> {
        Self::get_secret_key_entry(email.as_ref())
            .delete_secret()
            .map_err(Error::DeleteSecretKeyFromKeyringError)?;

        Self::get_public_key_entry(email.as_ref())
            .delete_secret()
            .map_err(Error::DeletePublicKeyFromKeyringError)?;

        Ok(())
    }

    /// Generates secret and public keys then stores them into the
    /// global keyring.
    pub async fn configure(&self, email: impl AsRef<str>) -> Result<()> {
        let (secret_key, public_key) = account::pgp::generate_key_pair(email.as_ref().to_owned())?;

        let secret_key = secret_key
            .to_armored_string(None)
            .map_err(Error::ExportSecretKeyToArmoredStringError)?;
        Self::get_secret_key_entry(email.as_ref())
            .set_secret(secret_key)
            .map_err(Error::SetSecretKeyToKeyringError)?;

        let public_key = public_key
            .to_armored_string(None)
            .map_err(Error::ExportPublicKeyToArmoredStringError)?;
        Self::get_public_key_entry(email.as_ref())
            .set_secret(public_key)
            .map_err(Error::SetPublicKeyToKeyringError)?;

        Ok(())
    }
}

/// The GPG configuration.
///
/// This configuration is based on the [`gpgme`] crate, which provides
/// bindings to the libgpgme.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpGpgConfig {
    //
}

/// The PGP commands configuration.
///
/// This configuration is based on system commands.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpCmdConfig {
    encrypt_cmd: Cmd,
    decrypt_cmd: Cmd,
    sign_cmd: Cmd,
    verify_cmd: Cmd,
}
