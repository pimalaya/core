#[cfg(feature = "gpg")]
pub mod gpg;
pub mod native;

use thiserror::Error;

use crate::Result;

#[cfg(feature = "gpg")]
pub use self::gpg::Gpg;
pub use self::native::{
    PgpNative, PgpNativePublicKeysResolver, PgpNativeSecretKey, SignedPublicKey, SignedSecretKey,
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
    Native(PgpNative),
    #[cfg(feature = "gpg")]
    Gpg(Gpg),
}

impl Pgp {
    pub async fn encrypt(
        &self,
        recipients: impl IntoIterator<Item = String>,
        plain_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        match self {
            Self::None => Ok(Err(Error::PgpEncryptNoneError)?),
            Self::Native(native) => native.encrypt(recipients, plain_bytes).await,
            #[cfg(feature = "gpg")]
            Self::Gpg(gpg) => gpg.encrypt(recipients, plain_bytes).await,
        }
    }

    pub async fn decrypt(
        &self,
        recipient: impl ToString,
        encrypted_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        match self {
            Self::None => Ok(Err(Error::PgpDecryptNoneError)?),
            Self::Native(native) => native.decrypt(recipient, encrypted_bytes).await,
            #[cfg(feature = "gpg")]
            Self::Gpg(gpg) => gpg.decrypt(encrypted_bytes).await,
        }
    }

    pub async fn sign(&self, recipient: impl ToString, plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
        match self {
            Self::None => Ok(Err(Error::PgpSignNoneError)?),
            Self::Native(native) => native.sign(recipient, plain_bytes).await,
            #[cfg(feature = "gpg")]
            Self::Gpg(gpg) => gpg.sign(plain_bytes).await,
        }
    }

    pub async fn verify(
        &self,
        recipient: impl AsRef<str>,
        signature_bytes: Vec<u8>,
        signed_bytes: Vec<u8>,
    ) -> Result<bool> {
        match self {
            Self::None => Ok(Err(Error::PgpVerifyNoneError)?),
            Self::Native(native) => {
                native
                    .verify(recipient, signature_bytes, signed_bytes)
                    .await
            }
            #[cfg(feature = "gpg")]
            Self::Gpg(gpg) => gpg.verify(signature_bytes, signed_bytes).await,
        }
    }
}
