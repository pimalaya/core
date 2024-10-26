use std::io;

use keyring::KeyringEntry;
use mml::pgp::{NativePgpPublicKeysResolver, NativePgpSecretKey, Pgp, PgpNative};
use secret::Secret;
use shellexpand_utils::shellexpand_path;
use tokio::fs;
use tracing::debug;

#[doc(inline)]
pub use super::{Error, Result};

/// The native PGP configuration.
///
/// This configuration is based on the [`pgp`] crate, which provides a
/// native Rust implementation of the PGP standard.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct PgpNativeConfig {
    pub secret_key: NativePgpSecretKey,
    pub secret_key_passphrase: Secret,
    pub wkd: bool,
    pub key_servers: Vec<String>,
}

impl PgpNativeConfig {
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
                let path = shellexpand_path(path);
                if path.is_file() {
                    fs::remove_file(&path)
                        .await
                        .map_err(|err| Error::DeletePgpKeyAtPathError(err, path.clone()))?;
                } else {
                    debug!("cannot delete pgp key file at {path:?}: file not found");
                }
            }
            #[cfg(feature = "keyring")]
            NativePgpSecretKey::Keyring(entry) => entry
                .delete_secret()
                .await
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
                let skey_path = shellexpand_path(skey_path);
                fs::write(&skey_path, skey)
                    .await
                    .map_err(|err| Error::WriteSecretKeyFileError(err, skey_path.clone()))?;

                let pkey_path = skey_path.with_extension("pub");
                fs::write(&pkey_path, pkey)
                    .await
                    .map_err(|err| Error::WritePublicKeyFileError(err, pkey_path))?;
            }
            NativePgpSecretKey::Keyring(skey_entry) => {
                let pkey_entry = KeyringEntry::try_new(skey_entry.key.clone() + "-pub")
                    .map_err(Error::GetPublicKeyFromKeyringError)?;

                skey_entry
                    .set_secret(skey)
                    .await
                    .map_err(Error::SetSecretKeyToKeyringError)?;
                pkey_entry
                    .set_secret(pkey)
                    .await
                    .map_err(Error::SetPublicKeyToKeyringError)?;
            }
        }

        Ok(())
    }
}

impl Default for PgpNativeConfig {
    fn default() -> Self {
        Self {
            secret_key: Default::default(),
            secret_key_passphrase: Default::default(),
            wkd: Self::default_wkd(),
            key_servers: Self::default_key_servers(),
        }
    }
}

impl From<PgpNativeConfig> for Pgp {
    fn from(config: PgpNativeConfig) -> Self {
        let public_keys_resolvers = {
            let mut resolvers = vec![];

            if config.wkd {
                resolvers.push(NativePgpPublicKeysResolver::Wkd)
            }

            resolvers.push(NativePgpPublicKeysResolver::KeyServers(config.key_servers));

            resolvers
        };

        Pgp::Native(PgpNative {
            secret_key: config.secret_key,
            secret_key_passphrase: config.secret_key_passphrase,
            public_keys_resolvers,
        })
    }
}
