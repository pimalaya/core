#[cfg(feature = "pgp-commands")]
pub mod cmds;
#[cfg(feature = "pgp-gpg")]
pub mod gpg;
#[cfg(feature = "pgp-native")]
pub mod native;

use log::{debug, trace};
use thiserror::Error;

use crate::Result;

#[cfg(feature = "pgp-commands")]
pub use self::cmds::CmdsPgp;
#[cfg(feature = "pgp-gpg")]
pub use self::gpg::Gpg;
#[cfg(feature = "pgp-native")]
pub use self::native::{
    NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, SignedPublicKey, SignedSecretKey,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot encrypt data using pgp: pgp not configured")]
    PgpEncryptNoneError,
    #[error("cannot decrypt data using pgp: pgp not configured")]
    PgpDecryptNoneError,
    #[error("cannot sign data using pgp: pgp not configured")]
    PgpSignNoneError,
    #[error("cannot verify data using pgp: pgp not configured")]
    PgpVerifyNoneError,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Pgp {
    #[default]
    None,
    #[cfg(feature = "pgp-commands")]
    Cmds(CmdsPgp),
    #[cfg(feature = "pgp-native")]
    Native(NativePgp),
    #[cfg(feature = "pgp-gpg")]
    Gpg(Gpg),
}

impl Pgp {
    pub async fn encrypt(
        &self,
        recipients: impl IntoIterator<Item = String>,
        plain_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        debug!("encrypting bytes using pgp");
        let plain_str = String::from_utf8_lossy(&plain_bytes);
        trace!("plain bytes: {plain_str}");

        match self {
            Self::None => Ok(Err(Error::PgpEncryptNoneError)?),
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(cmds) => cmds.encrypt(recipients, plain_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => native.encrypt(recipients, plain_bytes).await,
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.encrypt(recipients, plain_bytes).await,
        }
    }

    pub async fn decrypt(
        &self,
        recipient: impl ToString,
        encrypted_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        let recipient = recipient.to_string();
        debug!("decrypting bytes for {recipient} using pgp");
        let encrypted_str = String::from_utf8_lossy(&encrypted_bytes);
        trace!("encrypted bytes: {encrypted_str}");

        match self {
            Self::None => Ok(Err(Error::PgpDecryptNoneError)?),
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(cmds) => cmds.decrypt(encrypted_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => native.decrypt(recipient, encrypted_bytes).await,
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.decrypt(encrypted_bytes).await,
        }
    }

    pub async fn sign(&self, recipient: impl ToString, plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let recipient = recipient.to_string();
        debug!("signing bytes for {recipient} using pgp");
        let plain_str = String::from_utf8_lossy(&plain_bytes);
        trace!("plain bytes: {plain_str}");

        match self {
            Self::None => Ok(Err(Error::PgpSignNoneError)?),
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(cmds) => cmds.sign(plain_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => native.sign(recipient, plain_bytes).await,
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.sign(plain_bytes).await,
        }
    }

    pub async fn verify(
        &self,
        recipient: impl AsRef<str>,
        signature_bytes: Vec<u8>,
        signed_bytes: Vec<u8>,
    ) -> Result<()> {
        let recipient = recipient.as_ref();
        debug!("verifying signature for {recipient} using pgp");
        let signature_str = String::from_utf8_lossy(&signature_bytes);
        trace!("signature bytes: {signature_str}");
        let signed_str = String::from_utf8_lossy(&signed_bytes);
        trace!("signed bytes: {signed_str}");

        match self {
            Self::None => Ok(Err(Error::PgpVerifyNoneError)?),
            #[cfg(feature = "pgp-commands")]
            Self::Cmds(cmds) => cmds.verify(signature_bytes, signed_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => {
                native
                    .verify(recipient, signature_bytes, signed_bytes)
                    .await
            }
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.verify(signature_bytes, signed_bytes).await,
        }
    }
}
