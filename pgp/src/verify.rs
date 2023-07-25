//! Module dedicated to PGP verify.

use log::{debug, warn};
use pgp::{Deserializable, SignedPublicKey, StandaloneSignature};
use std::io::Cursor;
use thiserror::Error;

use crate::Result;

/// Errors related to PGP.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot trim carriage return and newline from pgp message")]
    TrimMessageCrLfError,
    #[error("cannot import pgp signature from armor")]
    ImportSignatureFromArmorError(#[source] pgp::errors::Error),
}

/// Verifies a standalone signature using the given public key.
pub fn verify(data: &[u8], sig: &[u8], pkey: &SignedPublicKey) -> Result<bool> {
    let sig = StandaloneSignature::from_armor_single(Cursor::new(sig))
        .map_err(Error::ImportSignatureFromArmorError)?
        .0;

    // Remove trailing CRLF before the delimiter.
    // According to RFC 3156 it is considered to be part of the MIME delimiter for the purpose of
    // OpenPGP signature calculation.
    // let data = data
    //     .get(..data.len().saturating_sub(2))
    //     .ok_or(Error::TrimMessageCrLfError)?;

    if let Err(err) = sig.verify(pkey, data) {
        warn!("cannot verify message signature: {err}");
        debug!("cannot verify message signature: {err:?}");
        Ok(false)
    } else {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use crate::{generate_key_pair, sign, verify};

    #[test]
    fn sign_then_verify() {
        let (skey, pkey) = generate_key_pair("test@localhost").unwrap();
        let msg = b"signed message";
        let sig = sign(msg, &skey).unwrap();

        assert_eq!(verify(msg, &sig, &pkey).unwrap(), true);
    }
}
