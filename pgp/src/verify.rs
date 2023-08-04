//! Module dedicated to PGP verify.

use log::{debug, warn};
use pgp::{SignedPublicKey, StandaloneSignature};
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to PGP.
#[derive(Debug, Error)]
pub enum Error {
    //
}

/// Verifies a standalone signature using the given public key.
pub async fn verify(
    pkey: SignedPublicKey,
    sig: StandaloneSignature,
    data: Vec<u8>,
) -> Result<bool> {
    task::spawn_blocking(move || {
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
    use crate::{gen_key_pair, read_sig_from_bytes, sign, verify};

    #[tokio::test]
    async fn sign_then_verify() {
        let (skey, pkey) = gen_key_pair("test@localhost", "").await.unwrap();
        let msg = b"signed message".to_vec();
        let raw_sig = sign(skey, "", msg.clone()).await.unwrap();
        let sig = read_sig_from_bytes(raw_sig).await.unwrap();

        assert_eq!(verify(pkey, sig, msg).await.unwrap(), true);
    }
}
