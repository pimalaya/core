//! # PGP
//!
//! This module contains available PGP backends: shell commands, GPG
//! and native.

#[cfg(feature = "pgp-commands")]
pub mod commands;
#[cfg(feature = "pgp-gpg")]
pub mod gpg;
#[cfg(feature = "pgp-native")]
pub mod native;

use tracing::{debug, trace};

use crate::{Error, Result};

#[cfg(feature = "pgp-commands")]
#[doc(inline)]
pub use self::commands::PgpCommands;
#[cfg(feature = "pgp-gpg")]
#[doc(inline)]
pub use self::gpg::PgpGpg;
#[cfg(feature = "pgp-native")]
#[doc(inline)]
pub use self::native::{
    NativePgpPublicKeysResolver, NativePgpSecretKey, PgpNative, SignedPublicKey, SignedSecretKey,
};

/// The PGP backends.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Pgp {
    #[default]
    None,

    /// Use shell commands to perform PGP actions.
    #[cfg(feature = "pgp-commands")]
    Commands(PgpCommands),

    /// Use GPG to perform PGP actions.
    ///
    /// GPG needs to be installed on the system as well as its
    /// associated library `gpgme`.
    #[cfg(feature = "pgp-gpg")]
    Gpg(PgpGpg),

    /// Use native Rust implementation of PGP to perform PGP actions.
    #[cfg(feature = "pgp-native")]
    Native(PgpNative),
}

impl Pgp {
    /// Encrypts the given plain bytes using the given recipients.
    pub async fn encrypt(
        &self,
        recipients: impl IntoIterator<Item = String>,
        plain_bytes: Vec<u8>,
    ) -> Result<Vec<u8>> {
        debug!("encrypting bytes using pgp");
        let plain_str = String::from_utf8_lossy(&plain_bytes);
        trace!("plain bytes: {plain_str}");

        match self {
            Self::None => Err(Error::PgpMissingConfigurationError),
            #[cfg(feature = "pgp-commands")]
            Self::Commands(cmds) => cmds.encrypt(recipients, plain_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => native.encrypt(recipients, plain_bytes).await,
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.encrypt(recipients, plain_bytes).await,
        }
    }

    /// Decrypts the given encrypted bytes using the given recipient.
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
            Self::None => Err(Error::PgpMissingConfigurationError),
            #[cfg(feature = "pgp-commands")]
            Self::Commands(cmds) => cmds.decrypt(encrypted_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => native.decrypt(recipient, encrypted_bytes).await,
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.decrypt(encrypted_bytes).await,
        }
    }

    /// Signs the given plain bytes using the given recipient.
    pub async fn sign(&self, recipient: impl ToString, plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
        let recipient = recipient.to_string();
        debug!("signing bytes for {recipient} using pgp");
        let plain_str = String::from_utf8_lossy(&plain_bytes);
        trace!("plain bytes: {plain_str}");

        match self {
            Self::None => Err(Error::PgpMissingConfigurationError),
            #[cfg(feature = "pgp-commands")]
            Self::Commands(cmds) => cmds.sign(plain_bytes).await,
            #[cfg(feature = "pgp-native")]
            Self::Native(native) => native.sign(recipient, plain_bytes).await,
            #[cfg(feature = "pgp-gpg")]
            Self::Gpg(gpg) => gpg.sign(plain_bytes).await,
        }
    }

    /// Verifies the given signed bytes as well as the given signature
    /// bytes using the given recipient.
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
            Self::None => Err(Error::PgpMissingConfigurationError),
            #[cfg(feature = "pgp-commands")]
            Self::Commands(cmds) => cmds.verify(signature_bytes, signed_bytes).await,
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
