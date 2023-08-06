//! Module dedicated to PGP signing.
//!
//! This module exposes a simple function [`sign`] and its associated
//! [`Error`]s.

use pgp::{crypto::hash::HashAlgorithm, Message, SignedSecretKey};
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to PGP signing.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot sign pgp message")]
    SignMessageError(#[source] pgp::errors::Error),
    #[error("cannot export signed pgp message as armored string")]
    ExportSignedMessageToArmoredBytesError(#[source] pgp::errors::Error),
}

/// Signs given bytes using the given private key and its passphrase.
pub async fn sign(
    skey: SignedSecretKey,
    passphrase: impl ToString,
    plain_bytes: Vec<u8>,
) -> Result<Vec<u8>> {
    let passphrase = passphrase.to_string();

    task::spawn_blocking(move || {
        let msg = Message::new_literal_bytes("", &plain_bytes)
            .sign(&skey, || passphrase, HashAlgorithm::SHA1)
            .map_err(Error::SignMessageError)?;

        let signature_bytes = msg
            .into_signature()
            .to_armored_bytes(None)
            .map_err(Error::ExportSignedMessageToArmoredBytesError)?;

        Ok(signature_bytes)
    })
    .await?
}
