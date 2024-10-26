//! # PGP native module
//!
//! This module contains the native PGP backend.

use std::{collections::HashSet, path::PathBuf};

pub use pgp::native::{SignedPublicKey, SignedSecretKey};
use secret::Secret;
use shellexpand_utils::shellexpand_path;
use tracing::debug;

use crate::{Error, Result};

/// The native PGP secret key source.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum NativePgpSecretKey {
    #[default]
    None,

    /// The native PGP secret key is given as it is (raw).
    #[cfg_attr(feature = "derive", serde(skip))]
    Raw(SignedSecretKey),

    /// The native PGP secret key is located at the given path.
    Path(PathBuf),

    #[cfg(feature = "keyring")]
    /// The native PGP secret key is located in the user's global
    /// keyring at the given entry.
    Keyring(secret::keyring::KeyringEntry),
}

impl NativePgpSecretKey {
    // FIXME: use the recipient from the template instead of the PGP
    // config. This can be done once the `pgp` module can manage both
    // secret and public keys.
    pub async fn get(&self, recipient: impl ToString) -> Result<SignedSecretKey> {
        let recipient = recipient.to_string();
        match self {
            Self::None => Ok(Err(Error::GetNativePgpSecretKeyNoneError(
                recipient.clone(),
            ))?),
            Self::Raw(skey) => Ok(skey.clone()),
            Self::Path(path) => {
                let path = shellexpand_path(path);
                let skey = pgp::read_skey_from_file(path)
                    .await
                    .map_err(Error::ReadNativePgpSecretKeyError)?;
                Ok(skey)
            }
            #[cfg(feature = "keyring")]
            Self::Keyring(entry) => {
                let data = entry
                    .get_secret()
                    .await
                    .map_err(Error::GetPgpSecretKeyFromKeyringError)?;
                let skey = pgp::read_skey_from_string(data)
                    .await
                    .map_err(Error::ReadNativePgpSecretKeyError)?;
                Ok(skey)
            }
        }
    }
}

/// The native PGP public key resolver.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum NativePgpPublicKeysResolver {
    /// The given email string is associated with the given raw public
    /// key.
    #[cfg_attr(feature = "derive", serde(skip))]
    Raw(String, SignedPublicKey),

    /// The public key is resolved using the Web Key Directory
    /// protocol.
    Wkd,

    /// The public key is resolved using the given key servers.
    ///
    /// Supported protocols: `http(s)://`, `hkp(s)://`.
    KeyServers(Vec<String>),
}

/// The native PGP backend.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct PgpNative {
    /// The secret key of the sender.
    pub secret_key: NativePgpSecretKey,

    /// The passphrase associated to the secret key.
    pub secret_key_passphrase: Secret,

    /// The list of public key resolvers.
    pub public_keys_resolvers: Vec<NativePgpPublicKeysResolver>,
}

impl PgpNative {
    /// Encrypts the given plain bytes using the given recipients.
    pub async fn encrypt(
        &self,
        emails: impl IntoIterator<Item = String>,
        data: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let mut pkeys = Vec::new();
        let mut recipients: HashSet<String> = HashSet::from_iter(emails.into_iter());

        for resolver in &self.public_keys_resolvers {
            match resolver {
                NativePgpPublicKeysResolver::Raw(recipient, pkey) => {
                    if recipients.remove(recipient) {
                        debug!("found pgp public key for {recipient} using raw pair");
                        pkeys.push(pkey.clone())
                    }
                }
                NativePgpPublicKeysResolver::Wkd => {
                    let recipients_clone = recipients.clone().into_iter().collect();
                    let wkd_pkeys = pgp::http::wkd::get_all(recipients_clone).await;

                    pkeys.extend(wkd_pkeys.into_iter().fold(
                        Vec::new(),
                        |mut pkeys, (ref recipient, res)| {
                            match res {
                                Ok(pkey) => {
                                    if recipients.remove(recipient) {
                                        debug!("found pgp public key for {recipient} using wkd");
                                        pkeys.push(pkey);
                                    }
                                }
                                Err(err) => {
                                    let msg = format!("cannot find pgp public key for {recipient}");
                                    debug!("{msg} using wkd: {err}");
                                    debug!("{err:?}");
                                }
                            }
                            pkeys
                        },
                    ));
                }
                NativePgpPublicKeysResolver::KeyServers(key_servers) => {
                    let recipients_clone = recipients.clone().into_iter().collect();
                    let http_pkeys =
                        pgp::http::get_all(recipients_clone, key_servers.to_owned()).await;

                    pkeys.extend(http_pkeys.into_iter().fold(
                        Vec::default(),
                        |mut pkeys, (ref recipient, res)| {
                            match res {
                                Ok(pkey) => {
                                    if recipients.remove(recipient) {
                                        let msg = format!("found pgp public key for {recipient}");
                                        debug!("{msg} using key servers");
                                        pkeys.push(pkey);
                                    }
                                }
                                Err(err) => {
                                    let msg = format!("cannot find pgp public key for {recipient}");
                                    debug!("{msg} using key servers: {err}");
                                    debug!("{err:?}");
                                }
                            }
                            pkeys
                        },
                    ));
                }
            }

            if recipients.is_empty() {
                break;
            }
        }

        let data = pgp::encrypt(pkeys, data)
            .await
            .map_err(Error::EncryptNativePgpError)?;

        Ok(data)
    }

    /// Decrypts the given encrypted bytes using the given recipient.
    pub async fn decrypt(&self, email: impl ToString, data: Vec<u8>) -> Result<Vec<u8>> {
        let skey = self.secret_key.get(email).await?;
        let passphrase = self
            .secret_key_passphrase
            .get()
            .await
            .map_err(Error::GetSecretKeyPassphraseFromKeyringError)?;
        let data = pgp::decrypt(skey, passphrase, data)
            .await
            .map_err(Error::DecryptNativePgpError)?;
        Ok(data)
    }

    /// Signs the given plain bytes using the given recipient.
    pub async fn sign(&self, email: impl ToString, data: Vec<u8>) -> Result<Vec<u8>> {
        let skey = self.secret_key.get(email).await?;
        let passphrase = self
            .secret_key_passphrase
            .get()
            .await
            .map_err(Error::GetSecretKeyPassphraseFromKeyringError)?;
        let data = pgp::sign(skey, passphrase, data)
            .await
            .map_err(Error::SignNativePgpError)?;
        Ok(data)
    }

    /// Verifies the given signed bytes as well as the signature bytes
    /// using the given recipient.
    pub async fn verify(&self, email: impl AsRef<str>, sig: Vec<u8>, data: Vec<u8>) -> Result<()> {
        let email = email.as_ref();
        let mut pkey_found = None;

        for resolver in &self.public_keys_resolvers {
            match resolver {
                NativePgpPublicKeysResolver::Raw(recipient, pkey) => {
                    if recipient == email {
                        debug!("found pgp public key for {recipient} using raw pair");
                        pkey_found = Some(pkey.clone());
                        break;
                    } else {
                        continue;
                    }
                }
                NativePgpPublicKeysResolver::Wkd => {
                    match pgp::http::wkd::get_one(email.to_owned()).await {
                        Ok(pkey) => {
                            debug!("found pgp public key for {email} using wkd");
                            pkey_found = Some(pkey);
                            break;
                        }
                        Err(err) => {
                            debug!(?err, "cannot find pgp public key for {email} using wkd");
                            continue;
                        }
                    }
                }
                NativePgpPublicKeysResolver::KeyServers(key_servers) => {
                    let pkey = pgp::http::get_one(email.to_owned(), key_servers.clone()).await;
                    match pkey {
                        Ok(pkey) => {
                            debug!("found pgp public key for {email} using key servers");
                            pkey_found = Some(pkey);
                            break;
                        }
                        Err(err) => {
                            let msg = format!("cannot find pgp public key for {email}");
                            debug!(?err, "{msg} using key servers");
                            continue;
                        }
                    }
                }
            }
        }

        let pkey = pkey_found.ok_or(Error::FindPgpPublicKeyError(email.to_owned()))?;
        let sig = pgp::read_sig_from_bytes(sig)
            .await
            .map_err(Error::ReadNativePgpSignatureError)?;
        pgp::verify(pkey, sig, data)
            .await
            .map_err(Error::VerifyNativePgpSignatureError)?;

        Ok(())
    }
}
