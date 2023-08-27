use keyring::Entry;
use log::{debug, warn};
use mml::{NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, Pgp};
use secret::Secret;
use std::{io, path::PathBuf};
use thiserror::Error;
use tokio::fs;

use crate::Result;

/// Errors related to PGP configuration.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot delete pgp key from keyring")]
    DeletePgpKeyFromKeyringError(#[source] keyring::Error),
    #[error("cannot delete pgp key at {1}")]
    DeletePgpKeyAtPathError(#[source] io::Error, PathBuf),
    #[error("cannot generate pgp key pair for {1}")]
    GeneratePgpKeyPairError(#[source] pgp::Error, String),
    #[error("cannot export secret key to armored string")]
    ExportSecretKeyToArmoredStringError(#[source] pgp::native::errors::Error),
    #[error("cannot export public key to armored string")]
    ExportPublicKeyToArmoredStringError(#[source] pgp::native::errors::Error),
    #[error("cannot write secret key file at {1}")]
    WriteSecretKeyFileError(#[source] io::Error, PathBuf),
    #[error("cannot write public key file at {1}")]
    WritePublicKeyFileError(#[source] io::Error, PathBuf),
    #[error("cannot set secret key to keyring")]
    SetSecretKeyToKeyringError(#[source] keyring::Error),
    #[error("cannot set public key to keyring")]
    SetPublicKeyToKeyringError(#[source] keyring::Error),
    #[error("cannot get secret key password")]
    GetPgpSecretKeyPasswdError(#[source] io::Error),
}

/// The native PGP configuration.
///
/// This configuration is based on the [`pgp`] crate, which provides a
/// native Rust implementation of the PGP standard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePgpConfig {
    pub secret_key: NativePgpSecretKey,
    pub secret_key_passphrase: Secret,
    pub wkd: bool,
    pub key_servers: Vec<String>,
}

impl NativePgpConfig {
    pub fn default_wkd() -> bool {
        true
    }

    pub fn default_key_servers() -> Vec<String> {
        vec![
            String::from("hkps://keys.openpgp.org"),
            String::from("hkps://keys.mailvelope.com"),
        ]
    }

    /// Deletes secret and public keys.
    pub async fn reset(&self) -> Result<()> {
        match &self.secret_key {
            NativePgpSecretKey::None => (),
            NativePgpSecretKey::Raw(..) => (),
            NativePgpSecretKey::Path(path) => {
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
            NativePgpSecretKey::Keyring(entry) => entry
                .delete_secret()
                .map_err(Error::DeletePgpKeyFromKeyringError)?,
        };

        Ok(())
    }

    /// Generates secret and public keys then stores them.
    pub async fn configure(
        &self,
        email: impl ToString,
        passwd: impl Fn() -> io::Result<String>,
    ) -> Result<()> {
        let email = email.to_string();
        let passwd = passwd().map_err(Error::GetPgpSecretKeyPasswdError)?;

        let (skey, pkey) = pgp::gen_key_pair(email.clone(), passwd)
            .await
            .map_err(|err| Error::GeneratePgpKeyPairError(err, email.clone()))?;
        let skey = skey
            .to_armored_string(None)
            .map_err(Error::ExportSecretKeyToArmoredStringError)?;
        let pkey = pkey
            .to_armored_string(None)
            .map_err(Error::ExportPublicKeyToArmoredStringError)?;

        match &self.secret_key {
            NativePgpSecretKey::None => (),
            NativePgpSecretKey::Raw(_) => (),
            NativePgpSecretKey::Path(skey_path) => {
                let skey_path = skey_path.to_string_lossy().to_string();
                let skey_path = match shellexpand::full(&skey_path) {
                    Ok(path) => PathBuf::from(path.to_string()),
                    Err(err) => {
                        warn!("cannot shell expand pgp secret key {skey_path}: {err}");
                        debug!("cannot shell expand pgp secret key {skey_path:?}: {err:?}");
                        PathBuf::from(skey_path)
                    }
                };
                fs::write(&skey_path, skey)
                    .await
                    .map_err(|err| Error::WriteSecretKeyFileError(err, skey_path.clone()))?;

                let pkey_path = skey_path.with_extension("pub");
                fs::write(&pkey_path, pkey)
                    .await
                    .map_err(|err| Error::WritePublicKeyFileError(err, pkey_path))?;
            }
            NativePgpSecretKey::Keyring(skey_entry) => {
                let pkey_entry = Entry::from(skey_entry.get_key().to_owned() + "-pub");

                skey_entry
                    .set_secret(skey)
                    .map_err(Error::SetSecretKeyToKeyringError)?;
                pkey_entry
                    .set_secret(pkey)
                    .map_err(Error::SetPublicKeyToKeyringError)?;
            }
        }

        Ok(())
    }
}

impl Default for NativePgpConfig {
    fn default() -> Self {
        Self {
            secret_key: Default::default(),
            secret_key_passphrase: Default::default(),
            wkd: Self::default_wkd(),
            key_servers: Self::default_key_servers(),
        }
    }
}

impl Into<Pgp> for NativePgpConfig {
    fn into(self) -> Pgp {
        let public_keys_resolvers = {
            let mut resolvers = vec![];

            if self.wkd {
                resolvers.push(NativePgpPublicKeysResolver::Wkd)
            }

            resolvers.push(NativePgpPublicKeysResolver::KeyServers(self.key_servers));

            resolvers
        };

        Pgp::Native(NativePgp {
            secret_key: self.secret_key,
            secret_key_passphrase: self.secret_key_passphrase,
            public_keys_resolvers,
        })
    }
}
