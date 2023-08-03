//! Module dedicated to PGP decryption.

use pgp::{Deserializable, Message, SignedSecretKey};
use std::io::Cursor;
use thiserror::Error;
use tokio::task;

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
pub async fn decrypt(
    data: Vec<u8>,
    skey: SignedSecretKey,
    passwd: impl ToString,
) -> Result<Vec<u8>> {
    let passwd = passwd.to_string();
    task::spawn_blocking(move || {
        let (msg, _) = Message::from_armor_single(Cursor::new(&data))
            .map_err(Error::ImportMessageFromArmorError)?;
        let (decryptor, _) = msg
            .decrypt(|| passwd, &[&skey])
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
    })
    .await?
}

#[cfg(test)]
mod tests {
    use crate::{decrypt, encrypt, generate_key_pair};

    #[tokio::test]
    async fn encrypt_then_decrypt() {
        let (alice_skey, alice_pkey) = generate_key_pair("alice@localhost", "").await.unwrap();
        let (bob_skey, bob_pkey) = generate_key_pair("bob@localhost", "").await.unwrap();
        let (carl_skey, _carl_pkey) = generate_key_pair("carl@localhost", "").await.unwrap();

        let msg = b"encrypted message".to_vec();
        let encrypted_msg = encrypt(msg.clone(), vec![alice_pkey, bob_pkey])
            .await
            .unwrap();

        let alice_msg = decrypt(encrypted_msg.clone(), alice_skey, "")
            .await
            .unwrap();
        assert_eq!(alice_msg, msg);

        let bob_msg = decrypt(encrypted_msg.clone(), bob_skey, "").await.unwrap();
        assert_eq!(bob_msg, msg);

        let carl_msg = decrypt(encrypted_msg.clone(), carl_skey, "")
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
