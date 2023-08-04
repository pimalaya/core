//! Module dedicated to PGP encryption.

use pgp::{
    crypto::hash::HashAlgorithm,
    types::{CompressionAlgorithm, KeyTrait, Mpi, PublicKeyTrait},
    Message, SignedPublicKey, SignedPublicSubKey,
};
use rand::{thread_rng, CryptoRng, Rng};
use std::io;
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to encryption.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot encrypt message using pgp")]
    EncryptMessageError(#[source] pgp::errors::Error),
    #[error("cannot export encrypted pgp message as armored string")]
    ExportEncryptedMessageToArmorError(#[source] pgp::errors::Error),
    #[error("cannot compress pgp message")]
    CompressMessageError(#[source] pgp::errors::Error),
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

/// Selects primary key or subkey to use for encryption.
///
/// First, tries to use subkeys. If none of the subkeys are suitable
/// for encryption, tries to use primary key. Returns `None` if the
/// public key cannot be used for encryption.
fn select_pkey_for_encryption(key: &SignedPublicKey) -> Option<SignedPublicKeyOrSubkey> {
    key.public_subkeys
        .iter()
        .find(|subkey| subkey.is_encryption_key())
        .map_or_else(
            move || {
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
pub async fn encrypt(pkeys: Vec<SignedPublicKey>, data: Vec<u8>) -> Result<Vec<u8>> {
    task::spawn_blocking(move || {
        let mut rng = thread_rng();

        let lit_msg = Message::new_literal_bytes("", &data);

        let pkeys: Vec<SignedPublicKeyOrSubkey> = pkeys
            .iter()
            .filter_map(select_pkey_for_encryption)
            .collect();
        let pkeys_refs: Vec<&SignedPublicKeyOrSubkey> = pkeys.iter().collect();

        let encrypted_msg = lit_msg
            .compress(CompressionAlgorithm::ZLIB)
            .map_err(Error::CompressMessageError)?
            .encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs)
            .map_err(Error::EncryptMessageError)?
            .to_armored_bytes(None)
            .map_err(Error::ExportEncryptedMessageToArmorError)?;

        Ok(encrypted_msg)
    })
    .await?
}
