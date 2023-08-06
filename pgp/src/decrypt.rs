//! Module dedicated to PGP decryption.
//!
//! This module exposes a simple function [`decrypt`] and its
//! associated [`Error`]s.

use pgp::{Deserializable, Message, SignedSecretKey};
use std::io::Cursor;
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to PGP decryption.
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
    #[error("cannot get empty pgp message content")]
    GetMessageContentEmptyError,
    #[error("cannot get empty pgp message")]
    GetMessageEmptyError,
}

/// Decrypts bytes using the given secret key and its passphrase.
pub async fn decrypt(
    skey: SignedSecretKey,
    passphrase: impl ToString,
    encrypted_bytes: Vec<u8>,
) -> Result<Vec<u8>> {
    let passphrase = passphrase.to_string();
    task::spawn_blocking(move || {
        let (msg, _) = Message::from_armor_single(Cursor::new(&encrypted_bytes))
            .map_err(Error::ImportMessageFromArmorError)?;
        let (decryptor, _) = msg
            .decrypt(|| passphrase, &[&skey])
            .map_err(Error::DecryptMessageError)?;
        let msgs = decryptor
            .collect::<pgp::errors::Result<Vec<_>>>()
            .map_err(Error::DecryptMessageError)?;
        let msg = msgs.into_iter().next().ok_or(Error::GetMessageEmptyError)?;
        let msg = msg.decompress().map_err(Error::DecompressMessageError)?;

        let plain_bytes = msg
            .get_content()
            .map_err(Error::GetMessageContentError)?
            .ok_or(Error::GetMessageContentEmptyError)?;

        Ok(plain_bytes)
    })
    .await?
}

#[cfg(test)]
mod tests {
    use crate::{decrypt, encrypt, gen_key_pair};

    #[tokio::test]
    async fn encrypt_then_decrypt() {
        let (alice_skey, alice_pkey) = gen_key_pair("alice@localhost", "").await.unwrap();
        let (bob_skey, bob_pkey) = gen_key_pair("bob@localhost", "").await.unwrap();
        let (carl_skey, _carl_pkey) = gen_key_pair("carl@localhost", "").await.unwrap();

        let msg = b"encrypted message".to_vec();
        let encrypted_msg = encrypt(vec![alice_pkey, bob_pkey], msg.clone())
            .await
            .unwrap();

        let alice_msg = decrypt(alice_skey, "", encrypted_msg.clone())
            .await
            .unwrap();
        assert_eq!(alice_msg, msg);

        let bob_msg = decrypt(bob_skey, "", encrypted_msg.clone()).await.unwrap();
        assert_eq!(bob_msg, msg);

        let carl_msg = decrypt(carl_skey, "", encrypted_msg.clone())
            .await
            .unwrap_err();
        assert!(matches!(
            carl_msg,
            crate::Error::DecryptError(super::Error::DecryptMessageError(
                pgp::errors::Error::MissingKey
            )),
        ));
    }
}
