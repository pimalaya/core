//! # Verify
//!
//! Module dedicated to PGP verification. This module exposes a simple
//! function [`verify`] and its associated [`Error`]s.

use crate::{
    native::{SignedPublicKey, StandaloneSignature},
    utils::spawn_blocking,
    Error, Result,
};

/// Verifies given standalone signature using the given public key.
pub async fn verify(
    pkey: SignedPublicKey,
    signature: StandaloneSignature,
    signed_bytes: Vec<u8>,
) -> Result<()> {
    spawn_blocking(move || {
        signature
            .verify(&pkey, &signed_bytes)
            .map_err(Error::VerifySignatureError)?;
        Ok(())
    })
    .await?
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "async-std")]
    use async_std::test;
    #[cfg(feature = "tokio")]
    use tokio::test;

    use crate::{gen_key_pair, read_sig_from_bytes, sign, verify};

    #[test_log::test(test)]
    async fn sign_then_verify() {
        let (skey, pkey) = gen_key_pair("test@localhost", "").await.unwrap();
        let msg = b"signed message".to_vec();
        let raw_sig = sign(skey, "", msg.clone()).await.unwrap();
        let sig = read_sig_from_bytes(raw_sig).await.unwrap();

        verify(pkey, sig, msg).await.unwrap();
    }
}
