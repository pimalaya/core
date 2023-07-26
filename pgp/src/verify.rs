//! Module dedicated to PGP verify.

use log::{debug, warn};
use pgp::{SignedPublicKey, StandaloneSignature};
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to PGP.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot trim carriage return and newline from pgp message")]
    TrimMessageCrLfError,
}

/// Verifies a standalone signature using the given public key.
pub async fn verify(
    data: Vec<u8>,
    sig: StandaloneSignature,
    pkey: SignedPublicKey,
) -> Result<bool> {
    task::spawn_blocking(move || {
        // Remove trailing CRLF before the delimiter.
        // According to RFC 3156 it is considered to be part of the MIME delimiter for the purpose of
        // OpenPGP signature calculation.
        // let data = data
        //     .get(..data.len().saturating_sub(2))
        //     .ok_or(Error::TrimMessageCrLfError)?;

        if let Err(err) = sig.verify(&pkey, &data) {
            warn!("cannot verify message signature: {err}");
            debug!("cannot verify message signature: {err:?}");
            Ok(false)
        } else {
            Ok(true)
        }
    })
    .await?
}

#[cfg(test)]
mod tests {
    use crate::{generate_key_pair, read_signature_from_bytes, sign, verify};

    #[tokio::test]
    async fn sign_then_verify() {
        let (skey, pkey) = generate_key_pair("test@localhost").await.unwrap();
        let msg = b"signed message".to_vec();
        let raw_sig = sign(msg.clone(), skey).await.unwrap();
        let sig = read_signature_from_bytes(raw_sig).await.unwrap();

        assert_eq!(verify(msg, sig, pkey).await.unwrap(), true);
    }
}
