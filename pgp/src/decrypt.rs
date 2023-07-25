//! Module dedicated to PGP decryption.

use pgp::{Deserializable, Message, SignedSecretKey};
use std::io::Cursor;
use thiserror::Error;

use crate::Result;

/// Errors related to PGP.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot import armored pgp message")]
    ImportMessageFromArmorError(#[source] pgp::errors::Error),
    #[error("cannot decrypt pgp message")]
    DecryptMessageError(#[source] pgp::errors::Error),
    #[error("cannot decompress pgp message")]
    DecompressMessageError(#[source] pgp::errors::Error),
    #[error("cannot get pgp message content")]
    GetMessageContentError(#[source] pgp::errors::Error),
    #[error("cannot get pgp message content: content is empty")]
    GetMessageContentEmptyError,
    #[error("cannot get pgp message")]
    GetMessageEmptyError,
}

/// Decrypts data using the given secret key.
pub fn decrypt(data: &[u8], skey: &SignedSecretKey) -> Result<Vec<u8>> {
    let cursor = Cursor::new(data);
    let (msg, _) =
        Message::from_armor_single(cursor).map_err(Error::ImportMessageFromArmorError)?;

    let (decryptor, _) = msg
        .decrypt(|| Default::default(), &[&skey])
        .map_err(Error::DecryptMessageError)?;
    let msgs = decryptor
        .collect::<pgp::errors::Result<Vec<_>>>()
        .map_err(Error::DecryptMessageError)?;

    let msg = msgs.into_iter().next().ok_or(Error::GetMessageEmptyError)?;
    let msg = msg.decompress().map_err(Error::DecompressMessageError)?;

    let content = msg
        .get_content()
        .map_err(Error::GetMessageContentError)?
        .ok_or(Error::GetMessageContentEmptyError)?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use crate::{decrypt, encrypt, generate_key_pair};

    #[test]
    fn encrypt_then_decrypt() {
        let (alice_skey, alice_pkey) = generate_key_pair("alice@localhost").unwrap();
        let (bob_skey, bob_pkey) = generate_key_pair("bob@localhost").unwrap();
        let (carl_skey, _carl_pkey) = generate_key_pair("carl@localhost").unwrap();

        let msg = b"encrypted message";
        let encrypted_msg = encrypt(msg, vec![&alice_pkey, &bob_pkey]).unwrap();

        assert_eq!(decrypt(&encrypted_msg, &alice_skey).unwrap(), msg);
        assert_eq!(decrypt(&encrypted_msg, &bob_skey).unwrap(), msg);
        assert!(matches!(
            decrypt(&encrypted_msg, &carl_skey).unwrap_err(),
            crate::Error::DecryptError(super::Error::DecryptMessageError(
                pgp::errors::Error::MissingKey
            )),
        ));
    }
}
