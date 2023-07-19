//! Module dedicated to PGP.
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
    ExportSignedMessageToArmorError(#[source] pgp::errors::Error),
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
    #[error("cannot trim carriage return and newline from pgp message")]
    TrimMessageCrLfError,
    #[error("cannot import pgp signature from armor")]
    ImportSignatureFromArmorError(#[source] pgp::errors::Error),
}

/// Wrapper around [`pgp`] public key types.
#[derive(Debug)]
enum SignedPublicKeyOrSubkey<'a> {
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

/// Select public key or subkey to use for encryption.
///
/// First, tries to use subkeys. If none of the subkeys are suitable
/// for encryption, tries to use primary key. Returns `None` if the public
/// key cannot be used for encryption.
///
/// TODO: take key flags and expiration dates into account
fn select_pk_for_encryption(key: &SignedPublicKey) -> Option<SignedPublicKeyOrSubkey> {
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

/// Encrypts `plain` text using `public_keys_for_encryption` and signs
/// it using `private_key_for_signing`.
pub fn encrypt(
    plain: &[u8],
    public_keys_for_encryption: Vec<SignedPublicKey>,
    private_key_for_signing: Option<SignedSecretKey>,
) -> Result<String> {
    let lit_msg = Message::new_literal_bytes("", plain);

    let pkeys: Vec<SignedPublicKeyOrSubkey> = public_keys_for_encryption
        .iter()
        .filter_map(select_pk_for_encryption)
        .collect();
    let pkeys_refs: Vec<&SignedPublicKeyOrSubkey> = pkeys.iter().collect();

    let mut rng = thread_rng();

    let encrypted_msg = if let Some(ref skey) = private_key_for_signing {
        lit_msg
            .sign(skey, || "".into(), Default::default())
            .and_then(|msg| msg.compress(CompressionAlgorithm::ZLIB))
            .and_then(|msg| msg.encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs))
    } else {
        lit_msg.encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs)
    };

    let msg = encrypted_msg.map_err(Error::EncryptMessageError)?;
    let encoded_msg = msg
        .to_armored_string(None)
        .map_err(Error::ExportEncryptedMessageToArmorError)?;

    Ok(encoded_msg)
}

/// Signs `plain` text using `private_key_for_signing`.
pub fn sign(plain: &[u8], private_key_for_signing: &SignedSecretKey) -> Result<String> {
    let msg = Message::new_literal_bytes("", plain)
        .sign(
            private_key_for_signing,
            || String::new(),
            Default::default(),
        )
        .map_err(Error::SignMessageError)?;

    let signature = msg
        .into_signature()
        .to_armored_string(None)
        .map_err(Error::ExportSignedMessageToArmorError)?;

    Ok(signature)
}

/// Decrypts the message with keys from the private key keyring.
///
/// Receiver private keys are provided in
/// `private_keys_for_decryption`.
///
/// Returns decrypted message and fingerprints
/// of all keys from the `public_keys_for_validation` keyring that
/// have valid signatures there.
pub fn decrypt(
    ctext: Vec<u8>,
    private_key_for_decryption: SignedSecretKey,
    public_keys_for_validation: &[SignedPublicKey],
) -> Result<Vec<u8>> {
    let cursor = Cursor::new(ctext);
    let (msg, _) =
        Message::from_armor_single(cursor).map_err(Error::ImportMessageFromArmorError)?;

    let (decryptor, _) = msg
        .decrypt(|| String::new(), &[&private_key_for_decryption])
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

    if let signed_msg @ Message::Signed { .. } = msg {
        for pkey in public_keys_for_validation {
            if let Err(err) = signed_msg.verify(&pkey.primary_key) {
                warn!("cannot verify message signature: {err}");
                debug!("cannot verify message signature: {err:?}");
            }
        }
    }

    Ok(content)
}

/// Verifies standalone signature.
pub fn verify(
    content: &[u8],
    signature: &[u8],
    public_keys_for_validation: &[SignedPublicKey],
) -> Result<()> {
    let standalone_signature = StandaloneSignature::from_armor_single(Cursor::new(signature))
        .map_err(Error::ImportSignatureFromArmorError)?
        .0;

    // Remove trailing CRLF before the delimiter.
    // According to RFC 3156 it is considered to be part of the MIME delimiter for the purpose of
    // OpenPGP signature calculation.
    let content = content
        .get(..content.len().saturating_sub(2))
        .ok_or(Error::TrimMessageCrLfError)?;

    for pkey in public_keys_for_validation {
        if let Err(err) = standalone_signature.verify(pkey, content) {
            warn!("cannot verify message signature: {err}");
            debug!("cannot verify message signature: {err:?}");
        }
    }

    Ok(())
}

/// The PGP keypair.
///
/// This has it's own struct to be able to keep the public and secret
/// keys together as they are one unit.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyPair {
    /// The email address.
    pub email: String,

    /// The signed public key.
    pub public: SignedPublicKey,

    /// the signed secret key.
    pub secret: SignedSecretKey,
}

impl KeyPair {
    /// Creates a new key pair from an email address.
    pub fn new(email: impl ToString) -> Result<Self> {
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

        Ok(Self {
            email: email.to_string(),
            secret: secret_key,
            public: public_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_key_pair() {
        let key_pair = KeyPair::new("test@localhost").unwrap();
        assert_eq!(key_pair.email, "test@localhost");
    }
}
