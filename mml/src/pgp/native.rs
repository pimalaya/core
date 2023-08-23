#[doc(inline)]
pub use pgp::{SignedPublicKey, SignedSecretKey};

use log::{debug, warn};
use pimalaya_keyring::Entry;
use pimalaya_secret::Secret;
use std::{collections::HashSet, path::PathBuf};
use thiserror::Error;

use crate::Result;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get pgp secret key from keyring")]
    GetSecretKeyFromKeyringError(pimalaya_keyring::Error),
    #[error("cannot read pgp secret key from keyring")]
    ReadSecretKeyFromKeyringError(pgp::Error),
    #[error("cannot read pgp secret key from path {1}")]
    ReadSecretKeyFromPathError(pgp::Error, PathBuf),

    #[error("cannot get pgp secret key passphrase from keyring")]
    GetSecretKeyPassphraseFromKeyringError(#[source] pimalaya_secret::Error),
    #[error("cannot get pgp secret key from keyring")]
    GetPgpSecretKeyFromKeyringError(#[source] pimalaya_keyring::Error),

    #[error("cannot get native pgp secret key of {0}")]
    GetNativePgpSecretKeyNoneError(String),
    #[error("cannot find native pgp public key of {0}")]
    FindPgpPublicKeyError(String),
    #[error("cannot encrypt data using native pgp")]
    EncryptNativePgpError(#[source] pgp::Error),
    #[error("cannot decrypt data using native pgp")]
    DecryptNativePgpError(#[source] pgp::Error),
    #[error("cannot sign data using native pgp")]
    SignNativePgpError(#[source] pgp::Error),
    #[error("cannot read native pgp signature")]
    ReadNativePgpSignatureError(#[source] pgp::Error),
    #[error("cannot verify native pgp signature")]
    VerifyNativePgpSignatureError(#[source] pgp::Error),
    #[error("cannot read native pgp secret key")]
    ReadNativePgpSecretKeyError(#[source] pgp::Error),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum NativePgpSecretKey {
    #[default]
    None,
    Raw(SignedSecretKey),
    Path(PathBuf),
    Keyring(Entry),
}

impl NativePgpSecretKey {
    // FIXME: use the recipient from the template instead of the PGP
    // config. This can be done once the `pgp` module can
    // manage both secret and public keys.
    pub async fn get(&self, recipient: impl ToString) -> Result<SignedSecretKey> {
        let recipient = recipient.to_string();
        match self {
            Self::None => Ok(Err(Error::GetNativePgpSecretKeyNoneError(
                recipient.clone(),
            ))?),
            Self::Raw(skey) => Ok(skey.clone()),
            Self::Path(path) => {
                let path = path.to_string_lossy().to_string();
                let path = match shellexpand::full(&path) {
                    Ok(path) => path.to_string(),
                    Err(err) => {
                        let msg = "cannot shell expand pgp secret key at";
                        warn!("{msg} {path}: {err}");
                        debug!("{msg} {path:?}: {err:?}");
                        path.to_owned()
                    }
                };
                let path = PathBuf::from(&path);
                let skey = pgp::read_skey_from_file(path)
                    .await
                    .map_err(Error::ReadNativePgpSecretKeyError)?;
                Ok(skey)
            }
            Self::Keyring(entry) => {
                let data = entry
                    .get_secret()
                    .map_err(Error::GetPgpSecretKeyFromKeyringError)?;
                let skey = pgp::read_skey_from_string(data)
                    .await
                    .map_err(Error::ReadNativePgpSecretKeyError)?;
                Ok(skey)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativePgpPublicKeysResolver {
    Raw(String, SignedPublicKey),
    Wkd,
    KeyServers(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NativePgp {
    pub secret_key: NativePgpSecretKey,
    pub secret_key_passphrase: Secret,
    pub public_keys_resolvers: Vec<NativePgpPublicKeysResolver>,
}

impl NativePgp {
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
                    let wkd_pkeys = pgp::wkd::get_all(recipients_clone).await;

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
                                    warn!("{msg} using wkd: {err}");
                                    debug!("{msg} using wkd: {err:?}");
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
                                    warn!("{msg} using key servers: {err}");
                                    debug!("{msg} using key servers: {err:?}");
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
                    let pkey = pgp::wkd::get_one(email.to_owned()).await;
                    match pkey {
                        Ok(pkey) => {
                            debug!("found pgp public key for {email} using wkd");
                            pkey_found = Some(pkey);
                            break;
                        }
                        Err(err) => {
                            let msg = format!("cannot find pgp public key for {email}");
                            warn!("{msg} using wkd: {err}");
                            debug!("{msg} using wkd: {err:?}");
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
                            warn!("{msg} using key servers: {err}");
                            debug!("{msg} using key servers: {err:?}");
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
