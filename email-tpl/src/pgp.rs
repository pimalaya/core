use log::{debug, warn};
use pimalaya_keyring::Entry;
use pimalaya_pgp::{SignedPublicKey, SignedSecretKey};
use pimalaya_secret::Secret;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};
use thiserror::Error;

use crate::Result;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get pgp secret key from keyring")]
    GetSecretKeyFromKeyringError(pimalaya_keyring::Error),
    #[error("cannot read pgp secret key from keyring")]
    ReadSecretKeyFromKeyringError(pimalaya_pgp::Error),
    #[error("cannot read pgp secret key from path {1}")]
    ReadSecretKeyFromPathError(pimalaya_pgp::Error, PathBuf),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PgpSecretKeyResolver {
    Raw(SignedSecretKey, Secret),
    Path(PathBuf, Secret),
    Keyring(Entry, Secret),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpSecretKey {
    #[default]
    Disabled,
    Enabled(Vec<PgpSecretKeyResolver>),
}

impl PgpSecretKey {
    // FIXME: use the sender from the template instead of the PGP
    // config. This can be done once the `pimalaya_pgp` module can
    // manage both secret and public keys.
    pub async fn get_skey(&self, _sender: String) -> Option<(SignedSecretKey, Secret)> {
        match self {
            Self::Disabled => {
                warn!("cannot get pgp secret key of {_sender}: resolvers disabled");
                None
            }
            Self::Enabled(resolvers) => {
                for resolver in resolvers {
                    match resolver {
                        PgpSecretKeyResolver::Raw(skey, passwd) => {
                            return Some((skey.clone(), passwd.clone()))
                        }
                        PgpSecretKeyResolver::Path(path, passwd) => {
                            if let Some(path) = path.as_path().to_str() {
                                let path_str = match shellexpand::full(path) {
                                    Ok(path) => path.to_string(),
                                    Err(err) => {
                                        let msg = "cannot shell expand pgp secret key";
                                        warn!("{msg} {path}: {err}");
                                        debug!("{msg} {path:?}: {err:?}");
                                        path.to_owned()
                                    }
                                };

                                let path = PathBuf::from(&path_str);

                                match pimalaya_pgp::read_signed_secret_key_from_path(path).await {
                                    Ok(skey) => return Some((skey, passwd.clone())),
                                    Err(err) => {
                                        warn!("cannot get pgp secret key at {path_str}: {err}");
                                        debug!("cannot get pgp secret key at {path_str}: {err:?}");
                                    }
                                }
                            }
                        }
                        PgpSecretKeyResolver::Keyring(entry, passwd) => {
                            let get_skey = || async {
                                let data = entry.get_secret()?;
                                let skey = pimalaya_pgp::read_skey_from_string(data).await?;
                                Result::Ok((skey, passwd.clone()))
                            };
                            match get_skey().await {
                                Ok(skey) => return Some(skey),
                                Err(err) => {
                                    warn!("cannot get pgp secret key from keyring: {err}");
                                    debug!("cannot get pgp secret key from keyring: {err:?}");
                                }
                            }
                        }
                    }
                }

                warn!("cannot find pgp secret key of {_sender}");
                None
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PgpPublicKeysResolver {
    Raw(HashMap<String, SignedPublicKey>),
    Wkd,
    KeyServers(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpPublicKeys {
    #[default]
    Disabled,
    Enabled(Vec<PgpPublicKeysResolver>),
}

impl FromIterator<PgpPublicKeysResolver> for PgpPublicKeys {
    fn from_iter<T: IntoIterator<Item = PgpPublicKeysResolver>>(iter: T) -> Self {
        Self::Enabled(iter.into_iter().collect())
    }
}

impl<T: IntoIterator<Item = PgpPublicKeysResolver>> From<Option<T>> for PgpPublicKeys {
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => Self::Disabled,
            Some(iter) => Self::from_iter(iter),
        }
    }
}

impl PgpPublicKeys {
    pub async fn get_pkeys(&self, recipients: Vec<String>) -> Option<Vec<SignedPublicKey>> {
        let mut pkeys = Vec::new();

        match self {
            Self::Disabled => {
                warn!("cannot get pgp public keys: resolvers disabled");
                None
            }
            Self::Enabled(resolvers) => {
                let mut recipients: HashSet<String> = HashSet::from_iter(recipients.into_iter());

                for resolver in resolvers {
                    match resolver {
                        PgpPublicKeysResolver::Raw(raws) => {
                            for (recipient, pkey) in raws {
                                if recipients.remove(recipient) {
                                    pkeys.push(pkey.clone())
                                }
                            }
                        }
                        PgpPublicKeysResolver::Wkd => {
                            let recpts: Vec<_> = recipients.clone().into_iter().collect();
                            let wkd_pkeys = pimalaya_pgp::wkd::get_all(recpts).await;

                            pkeys.extend(wkd_pkeys.into_iter().fold(
                                Vec::new(),
                                |mut pkeys, (ref email, res)| {
                                    match res {
                                        Ok(pkey) => {
                                            if recipients.remove(email) {
                                                pkeys.push(pkey);
                                            }
                                        }
                                        Err(err) => {
                                            let msg =
                                                format!("cannot get pgp public key for {email}");
                                            warn!("{msg} using wkd: {err}");
                                            debug!("{msg} using wkd: {err:?}");
                                        }
                                    }
                                    pkeys
                                },
                            ));
                        }
                        PgpPublicKeysResolver::KeyServers(key_servers) => {
                            let recpts: Vec<_> = recipients.clone().into_iter().collect();
                            let http_pkeys =
                                pimalaya_pgp::http::get_all(recpts, key_servers.to_owned()).await;

                            pkeys.extend(http_pkeys.into_iter().fold(
                                Vec::default(),
                                |mut pkeys, (ref email, res)| {
                                    match res {
                                        Ok(pkey) => {
                                            if recipients.remove(email) {
                                                pkeys.push(pkey);
                                            }
                                        }
                                        Err(err) => {
                                            let msg =
                                                format!("cannot get pgp public key for {email}");
                                            warn!("{msg} using http: {err}");
                                            debug!("{msg} using http: {err:?}");
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

                if !recipients.is_empty() {
                    let emails_len = recipients.len();
                    let emails = recipients.into_iter().collect::<Vec<_>>().join(", ");
                    warn!("cannot get pgp public key of {emails_len} emails");
                    debug!("cannot get pgp public key of {emails_len} emails: {emails}");
                }

                Some(pkeys)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PgpPublicKeyResolver {
    Raw(SignedPublicKey),
    Wkd,
    KeyServers(Vec<String>),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum PgpPublicKey {
    #[default]
    Disabled,
    Enabled(Vec<PgpPublicKeyResolver>),
}

impl FromIterator<PgpPublicKeyResolver> for PgpPublicKey {
    fn from_iter<T: IntoIterator<Item = PgpPublicKeyResolver>>(iter: T) -> Self {
        Self::Enabled(iter.into_iter().collect())
    }
}

impl<T: IntoIterator<Item = PgpPublicKeyResolver>> From<Option<T>> for PgpPublicKey {
    fn from(opt: Option<T>) -> Self {
        match opt {
            None => Self::Disabled,
            Some(iter) => Self::from_iter(iter),
        }
    }
}

impl PgpPublicKey {
    // FIXME: use the sender from the template instead of the PGP
    // config. This can be done once the `pimalaya_pgp` module can
    // manage both public and public keys.
    pub async fn get_pkey(&self, recipient: String) -> Option<SignedPublicKey> {
        match self {
            Self::Disabled => {
                warn!("cannot get pgp public key for {recipient}: resolvers disabled");
                None
            }
            Self::Enabled(resolvers) => {
                for resolver in resolvers {
                    match resolver {
                        PgpPublicKeyResolver::Raw(pkey) => return Some(pkey.clone()),
                        PgpPublicKeyResolver::Wkd => {
                            let pkey = pimalaya_pgp::wkd::get_one(recipient.clone()).await;
                            match pkey {
                                Ok(pkey) => return Some(pkey),
                                Err(err) => {
                                    let msg = format!("cannot get pgp public key for {recipient}");
                                    warn!("{msg} using wkd: {err}");
                                    debug!("{msg} using wkd: {err:?}");
                                }
                            }
                        }
                        PgpPublicKeyResolver::KeyServers(key_servers) => {
                            let pkey =
                                pimalaya_pgp::http::get_one(recipient.clone(), key_servers.clone())
                                    .await;
                            match pkey {
                                Ok(pkey) => return Some(pkey),
                                Err(err) => {
                                    let msg = format!("cannot get pgp public key for {recipient}");
                                    warn!("{msg} using http: {err}");
                                    debug!("{msg} using http: {err:?}");
                                }
                            }
                        }
                    }
                }

                warn!("cannot find pgp public key for {recipient}");
                None
            }
        }
    }
}
