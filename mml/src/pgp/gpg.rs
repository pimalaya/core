use gpgme::{Context, Protocol};
use log::{debug, trace, warn};
use std::path::PathBuf;
use thiserror::Error;

use crate::Result;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot get gpg context")]
    GetContextError(#[source] gpgme::Error),
    #[error("cannot get gpg home dir path from {0}")]
    GetHomeDirPathError(PathBuf),
    #[error("cannot set gpg home dir at {1}")]
    SetHomeDirError(#[source] gpgme::Error, PathBuf),
    #[error("cannot encrypt data using gpg")]
    EncryptError(#[source] gpgme::Error),
    #[error("cannot decrypt data using gpg")]
    DecryptError(#[source] gpgme::Error),
    #[error("cannot sign data using gpg")]
    SignError(#[source] gpgme::Error),
    #[error("cannot verify data using gpg")]
    VerifyError(#[source] gpgme::Error),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Gpg {
    pub home_dir: Option<PathBuf>,
}

impl Gpg {
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

        Ok(ctx)
    }

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
                    warn!("cannot locate gpg key for {email}: {err}");
                    debug!("cannot locate gpg key for {email}: {err}");
                }
            }
        }

        let mut encrypted_bytes = Vec::new();
        let res = ctx
            .encrypt(keys.iter(), plain_bytes, &mut encrypted_bytes)
            .map_err(Error::EncryptError)?;
        trace!("encrypt result: {res:#?}");

        let recipients_count = res.invalid_recipients().count();
        if recipients_count > 0 {
            warn!("skipping {recipients_count} recipients from gpg encryption");
            debug!("invalid recipients: {:#?}", res.invalid_recipients());
        }

        Ok(encrypted_bytes)
    }

    pub async fn decrypt(&self, mut encrypted_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut ctx = self.get_context()?;

        let mut plain_bytes = Vec::new();
        let res = ctx
            .decrypt(&mut encrypted_bytes, &mut plain_bytes)
            .map_err(Error::DecryptError)?;
        trace!("decrypt result: {res:#?}");

        Ok(plain_bytes)
    }

    pub async fn sign(&self, mut plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let mut ctx = self.get_context()?;

        let mut signed_bytes = Vec::new();
        let res = ctx
            .sign_clear(&mut plain_bytes, &mut signed_bytes)
            .map_err(Error::SignError)?;
        trace!("sign result: {res:#?}");

        Ok(signed_bytes)
    }

    pub async fn verify(&self, signature_bytes: Vec<u8>, signed_bytes: Vec<u8>) -> Result<()> {
        let mut ctx = self.get_context()?;

        let res = ctx
            .verify_opaque(signature_bytes, signed_bytes)
            .map_err(Error::SignError)?;
        trace!("verify result: {res:#?}");

        Ok(())
    }
}
