//! Module dedicated to PGP verification.
//!
//! This module exposes a simple function [`verify`] and its
//! associated [`Error`]s.

use pgp_native::{SignedPublicKey, StandaloneSignature};
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to PGP verification.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot verify pgp signature")]
    VerifySignatureError(#[source] pgp_native::errors::Error),
}

/// Verifies given standalone signature using the given public key.
pub async fn verify(
    pkey: SignedPublicKey,
    signature: StandaloneSignature,
    signed_bytes: Vec<u8>,
) -> Result<()> {
    task::spawn_blocking(move || {
        signature
            .verify(&pkey, &signed_bytes)
            .map_err(Error::VerifySignatureError)?;
        Ok(())
    })
    .await?
}

#[cfg(test)]
mod tests {
    use crate::{gen_key_pair, read_sig_from_bytes, sign, verify};

    #[tokio::test]
    async fn sign_then_verify() {
        let (skey, pkey) = gen_key_pair("test@localhost", "").await.unwrap();
        let msg = b"signed message".to_vec();
        let raw_sig = sign(skey, "", msg.clone()).await.unwrap();
        let sig = read_sig_from_bytes(raw_sig).await.unwrap();

        assert_eq!(verify(pkey, sig, msg).await.unwrap(), ());
    }
}
