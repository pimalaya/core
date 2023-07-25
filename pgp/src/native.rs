//! Module dedicated to the native implementation of PGP.
//!
//! This module contains utility functions around [`pgp`]. Inspired by
//! [deltachat](https://github.com/deltachat/deltachat-core-rust/blob/6d37e8601e51c29b40681ecb6606e8e023959e61/src/pgp.rs).

use log::{debug, warn};
use pgp::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::{CompressionAlgorithm, KeyTrait, Mpi, PublicKeyTrait, SecretKeyTrait},
    Deserializable, KeyType, Message, SecretKeyParamsBuilder, SecretKeyParamsBuilderError,
    SignedPublicKey, SignedPublicSubKey, SignedSecretKey, StandaloneSignature, SubkeyParamsBuilder,
    SubkeyParamsBuilderError,
};
use pimalaya_keyring::Entry;
use rand::{thread_rng, CryptoRng, Rng};
use smallvec::smallvec;
use std::io::{self, Cursor};
use thiserror::Error;

use crate::Result;

/// Errors related to PGP.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build pgp secret key params")]
    BuildSecretKeyParamsError(#[source] SecretKeyParamsBuilderError),
    #[error("cannot generate pgp secret key")]
    GenerateSecretKeyError(#[source] pgp::errors::Error),
    #[error("cannot sign pgp secret key")]
    SignSecretKeyError(#[source] pgp::errors::Error),
    #[error("cannot verify pgp secret key")]
    VerifySecretKeyError(#[source] pgp::errors::Error),

    #[error("cannot build pgp public subkey")]
    BuildSubkeyParamsError(#[from] SubkeyParamsBuilderError),
    #[error("cannot sign pgp public subkey")]
    SignPublicKeyError(#[source] pgp::errors::Error),
    #[error("cannot verify pgp public key")]
    VerifyPublicKeyError(#[source] pgp::errors::Error),

    #[error("cannot encrypt message using pgp")]
    EncryptMessageError(#[source] pgp::errors::Error),
    #[error("cannot export encrypted pgp message as armored string")]
    ExportEncryptedMessageToArmorError(#[source] pgp::errors::Error),
    #[error("cannot import armored pgp message")]
    ImportMessageFromArmorError(#[source] pgp::errors::Error),
    #[error("cannot sign pgp message")]
    SignMessageError(#[source] pgp::errors::Error),
    #[error("cannot export signed pgp message as armored string")]
    ExportSignedMessageToArmoredBytesError(#[source] pgp::errors::Error),
    #[error("cannot decrypt pgp message")]
    DecryptMessageError(#[source] pgp::errors::Error),
    #[error("cannot compress pgp message")]
    CompressMessageError(#[source] pgp::errors::Error),
    #[error("cannot decompress pgp message")]
    DecompressMessageError(#[source] pgp::errors::Error),
    #[error("cannot get pgp message content")]
    GetMessageContentError(#[source] pgp::errors::Error),
    #[error("cannot get pgp message content: content is empty")]
    GetMessageContentEmptyError,
    #[error("cannot get pgp message")]
    GetMessageEmptyError,
    #[error("cannot trim carriage return and newline from pgp message")]
    TrimMessageCrLfError,
    #[error("cannot import pgp signature from armor")]
    ImportSignatureFromArmorError(#[source] pgp::errors::Error),

    #[error("cannot find pgp secret key for address {1}")]
    GetSecretKeyError(#[source] pimalaya_keyring::Error, String),
    #[error("cannot find pgp secret key for address {0}")]
    GetSecretKeyNotFoundError(String),
    #[error("cannot get pgp secret key for address {1}")]
    GetSecretKeyFromKeyringError(#[source] pgp::errors::Error, String),
}

/// Wrapper around [`pgp`] public key types.
#[derive(Debug)]
pub enum SignedPublicKeyOrSubkey<'a> {
    Key(&'a SignedPublicKey),
    Subkey(&'a SignedPublicSubKey),
}

impl KeyTrait for SignedPublicKeyOrSubkey<'_> {
    fn fingerprint(&self) -> Vec<u8> {
        match self {
            Self::Key(k) => k.fingerprint(),
            Self::Subkey(k) => k.fingerprint(),
        }
    }

    fn key_id(&self) -> pgp::types::KeyId {
        match self {
            Self::Key(k) => k.key_id(),
            Self::Subkey(k) => k.key_id(),
        }
    }

    fn algorithm(&self) -> pgp::crypto::public_key::PublicKeyAlgorithm {
        match self {
            Self::Key(k) => k.algorithm(),
            Self::Subkey(k) => k.algorithm(),
        }
    }
}

impl PublicKeyTrait for SignedPublicKeyOrSubkey<'_> {
    fn verify_signature(
        &self,
        hash: HashAlgorithm,
        data: &[u8],
        sig: &[Mpi],
    ) -> pgp::errors::Result<()> {
        match self {
            Self::Key(k) => k.verify_signature(hash, data, sig),
            Self::Subkey(k) => k.verify_signature(hash, data, sig),
        }
    }

    fn encrypt<R: Rng + CryptoRng>(
        &self,
        rng: &mut R,
        plain: &[u8],
    ) -> pgp::errors::Result<Vec<Mpi>> {
        match self {
            Self::Key(k) => k.encrypt(rng, plain),
            Self::Subkey(k) => k.encrypt(rng, plain),
        }
    }

    fn to_writer_old(&self, writer: &mut impl io::Write) -> pgp::errors::Result<()> {
        match self {
            Self::Key(k) => k.to_writer_old(writer),
            Self::Subkey(k) => k.to_writer_old(writer),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PgpNative {}

impl PgpNative {
    pub fn encrypt(&self, _data: &[u8], _receivers: &[impl ToString]) -> Result<Vec<u8>> {
        unimplemented!()
    }

    pub fn decrypt(&self, _data: &[u8], _senders: &[impl ToString]) -> Result<Vec<u8>> {
        unimplemented!()
    }

    pub fn sign(&self, data: &[u8], sender: impl ToString) -> Result<Vec<u8>> {
        let secret_key = Entry::from(sender.to_string())
            .find_secret()
            .map_err(|err| Error::GetSecretKeyError(err, sender.to_string()))?
            .ok_or_else(|| Error::GetSecretKeyNotFoundError(sender.to_string()))?;
        let secret_key = SignedSecretKey::from_bytes(secret_key.as_bytes())
            .map_err(|err| Error::GetSecretKeyFromKeyringError(err, sender.to_string()))?;

        let signature = sign(data, &secret_key)?;

        Ok(signature)
    }

    pub fn verify(&self, _data: &[u8], _receiver: impl ToString) -> Result<Vec<u8>> {
        unimplemented!()
    }
}

/// Creates a new key pair from an email address.
pub(super) fn generate_key_pair(
    email: impl ToString,
) -> Result<(SignedSecretKey, SignedPublicKey)> {
    let key_params = SecretKeyParamsBuilder::default()
        .key_type(KeyType::EdDSA)
        .can_create_certificates(true)
        .can_sign(true)
        .primary_user_id(email.to_string())
        .passphrase(None)
        .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256])
        .preferred_hash_algorithms(smallvec![HashAlgorithm::SHA2_512])
        .preferred_compression_algorithms(smallvec![CompressionAlgorithm::ZLIB])
        .subkey(
            SubkeyParamsBuilder::default()
                .key_type(KeyType::ECDH)
                .can_encrypt(true)
                .passphrase(None)
                .build()
                .map_err(Error::BuildSubkeyParamsError)?,
        )
        .build()
        .map_err(Error::BuildSecretKeyParamsError)?;

    let secret_key = key_params
        .generate()
        .map_err(Error::GenerateSecretKeyError)?;
    let secret_key = secret_key
        .sign(|| String::new())
        .map_err(Error::SignSecretKeyError)?;
    secret_key.verify().map_err(Error::VerifySecretKeyError)?;

    let public_key = secret_key.public_key();
    let public_key = public_key
        .sign(&secret_key, || String::new())
        .map_err(Error::SignPublicKeyError)?;
    public_key.verify().map_err(Error::VerifyPublicKeyError)?;

    Ok((secret_key, public_key))
}

/// Select public key or subkey to use for encryption.
///
/// First, tries to use subkeys. If none of the subkeys are suitable
/// for encryption, tries to use primary key. Returns `None` if the
/// public key cannot be used for encryption.
///
/// TODO: take key flags and expiration dates into account
fn select_pkey_for_encryption(key: &SignedPublicKey) -> Option<SignedPublicKeyOrSubkey> {
    key.public_subkeys
        .iter()
        .find(|subkey| subkey.is_encryption_key())
        .map_or_else(
            || {
                // No usable subkey found, try primary key
                if key.is_encryption_key() {
                    Some(SignedPublicKeyOrSubkey::Key(key))
                } else {
                    None
                }
            },
            |subkey| Some(SignedPublicKeyOrSubkey::Subkey(subkey)),
        )
}

/// Encrypts data using the given public keys.
pub(super) fn encrypt(data: &[u8], pkeys: Vec<&SignedPublicKey>) -> Result<Vec<u8>> {
    let mut rng = thread_rng();
    let lit_msg = Message::new_literal_bytes("", data);

    let pkeys: Vec<SignedPublicKeyOrSubkey> = pkeys
        .into_iter()
        .filter_map(select_pkey_for_encryption)
        .collect();
    let pkeys_refs: Vec<&SignedPublicKeyOrSubkey> = pkeys.iter().collect();

    // let encrypted_msg = if let Some(ref skey) = private_key_for_signing {
    //     lit_msg
    //         .sign(skey, || "".into(), Default::default())
    //         .and_then(|msg| msg.compress(CompressionAlgorithm::ZLIB))
    //         .and_then(|msg| msg.encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs))
    // } else {
    //     lit_msg.encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs)
    // };

    let encrypted_msg = lit_msg
        .compress(CompressionAlgorithm::ZLIB)
        .map_err(Error::CompressMessageError)?
        .encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs)
        .map_err(Error::EncryptMessageError)?
        .to_armored_bytes(None)
        .map_err(Error::ExportEncryptedMessageToArmorError)?;

    Ok(encrypted_msg)
}

/// Decrypts the message using the given secret key.
pub(super) fn decrypt(data: &[u8], skey: &SignedSecretKey) -> Result<Vec<u8>> {
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

    // if let signed_msg @ Message::Signed { .. } = msg {
    //     for pkey in public_keys_for_validation {
    //         if let Err(err) = signed_msg.verify(&pkey.primary_key) {
    //             warn!("cannot verify message signature: {err}");
    //             debug!("cannot verify message signature: {err:?}");
    //         }
    //     }
    // }

    Ok(content)
}

/// Signs data using the given private key.
pub(super) fn sign(data: &[u8], skey: &SignedSecretKey) -> Result<Vec<u8>> {
    let msg = Message::new_literal_bytes("", data)
        .sign(&skey, || Default::default(), Default::default())
        .map_err(Error::SignMessageError)?;

    let sig = msg
        .into_signature()
        .to_armored_bytes(None)
        .map_err(Error::ExportSignedMessageToArmoredBytesError)?;

    Ok(sig)
}

/// Verifies a standalone signature using the given public key.
pub(crate) fn verify(data: &[u8], sig: &[u8], pkey: &SignedPublicKey) -> Result<bool> {
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
    #[test]
    fn encrypt_then_decrypt() {
        let (alice_skey, alice_pkey) = super::generate_key_pair("alice@localhost").unwrap();
        let (bob_skey, bob_pkey) = super::generate_key_pair("bob@localhost").unwrap();
        let (carl_skey, _carl_pkey) = super::generate_key_pair("carl@localhost").unwrap();

        let msg = b"encrypted message";
        let encrypted_msg = super::encrypt(msg, vec![&alice_pkey, &bob_pkey]).unwrap();

        assert_eq!(super::decrypt(&encrypted_msg, &alice_skey).unwrap(), msg);
        assert_eq!(super::decrypt(&encrypted_msg, &bob_skey).unwrap(), msg);
        assert!(matches!(
            super::decrypt(&encrypted_msg, &carl_skey).unwrap_err(),
            crate::Error::NativeError(super::Error::DecryptMessageError(
                pgp::errors::Error::MissingKey
            )),
        ));
    }

    #[test]
    fn sign_then_verify() {
        let (skey, pkey) = super::generate_key_pair("test@localhost").unwrap();
        let msg = b"signed message";
        let sig = super::sign(msg, &skey).unwrap();

        assert_eq!(super::verify(msg, &sig, &pkey).unwrap(), true);
    }
}
