//! # PGP GPG module
//!
//! This module contains the PGP backend based on GPG.

use std::path::PathBuf;

use gpgme::{Context, Protocol};
use tracing::{debug, trace};

use crate::{Error, Result};

/// The GPG PGP backend.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct PgpGpg {
    /// The GPG home directory.
    ///
    /// Defaults to GPG default home directory (~/.gpg).
    pub home_dir: Option<PathBuf>,
}

impl PgpGpg {
    pub fn get_context(&self) -> Result<Context> {
        let mut ctx = Context::from_protocol(Protocol::OpenPgp).map_err(Error::GetContextError)?;

        if let Some(path) = &self.home_dir {
            let home_dir = path
                .as_path()
                .to_str()
                .ok_or_else(|| Error::GetHomeDirPathError(path.clone()))?;

            ctx.set_engine_home_dir(home_dir)
                .map_err(|err| Error::SetHomeDirError(err, path.clone()))?;
        }

        ctx.set_armor(true);

        Ok(ctx)
    }

    /// Encrypts the given plain bytes using the given recipients.
    pub async fn encrypt(
        &self,
        emails: impl IntoIterator<Item = String>,
        plain_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let mut ctx = self.get_context()?;

        // TODO: make it really async
        let mut keys = Vec::new();
        for ref email in emails {
            match ctx.locate_key(email) {
                Ok(key) => {
                    debug!("found public key for {email} for encryption");
                    trace!("{key:#?}");
                    keys.push(key);
                }
                Err(err) => {
                    debug!("cannot locate gpg key for {email}: {err}");
                    debug!("cannot locate gpg key for {email}: {err}");
                }
            }
        }

        let mut encrypted_bytes = Vec::new();
        let res = ctx
            .encrypt(keys.iter(), plain_bytes, &mut encrypted_bytes)
            .map_err(Error::EncryptGpgError)?;
        trace!("encrypt result: {res:#?}");

        let recipients_count = res.invalid_recipients().count();
        if recipients_count > 0 {
            debug!("skipping {recipients_count} recipients from gpg encryption");
            debug!("invalid recipients: {:#?}", res.invalid_recipients());
        }

        Ok(encrypted_bytes)
    }

    /// Decrypts the given encrypted bytes.
    pub async fn decrypt(&self, mut encrypted_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut ctx = self.get_context()?;

        let mut plain_bytes = Vec::new();
        let res = ctx
            .decrypt(&mut encrypted_bytes, &mut plain_bytes)
            .map_err(Error::DecryptGpgError)?;
        trace!("decrypt result: {res:#?}");

        Ok(plain_bytes)
    }

    /// Signs the given plain bytes.
    pub async fn sign(&self, mut plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut ctx = self.get_context()?;

        let mut signed_bytes = Vec::new();
        let res = ctx
            .sign_clear(&mut plain_bytes, &mut signed_bytes)
            .map_err(Error::SignGpgError)?;
        trace!("sign result: {res:#?}");

        Ok(signed_bytes)
    }

    /// Verifies the given signed bytes as well as the signature_bytes.
    pub async fn verify(&self, signature_bytes: Vec<u8>, signed_bytes: Vec<u8>) -> Result<()> {
        let mut ctx = self.get_context()?;

        let res = ctx
            .verify_opaque(signature_bytes, signed_bytes)
            .map_err(Error::SignGpgError)?;
        trace!("verify result: {res:#?}");

        Ok(())
    }
}
