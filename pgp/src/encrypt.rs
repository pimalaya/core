//! # Encrypt
//!
//! Module dedicated to PGP encryption. This module exposes a simple
//! function [`encrypt`] and its associated [`Error`]s.

use std::io;

use rand::{thread_rng, CryptoRng, Rng};

use crate::{
    native::{
        self,
        crypto::{hash::HashAlgorithm, public_key::PublicKeyAlgorithm},
        types::{CompressionAlgorithm, KeyId, KeyTrait, Mpi, PublicKeyTrait},
        Message, SignedPublicKey, SignedPublicSubKey,
    },
    utils::spawn_blocking,
    Error, Result,
};

/// Wrapper around [`pgp`] public key types.
///
/// This enum is used to find the right encryption-capable public
/// (sub)key.
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

    fn key_id(&self) -> KeyId {
        match self {
            Self::Key(k) => k.key_id(),
            Self::Subkey(k) => k.key_id(),
        }
    }

    fn algorithm(&self) -> PublicKeyAlgorithm {
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
    ) -> native::errors::Result<()> {
        match self {
            Self::Key(k) => k.verify_signature(hash, data, sig),
            Self::Subkey(k) => k.verify_signature(hash, data, sig),
        }
    }

    fn encrypt<R: Rng + CryptoRng>(
        &self,
        rng: &mut R,
        plain: &[u8],
    ) -> native::errors::Result<Vec<Mpi>> {
        match self {
            Self::Key(k) => k.encrypt(rng, plain),
            Self::Subkey(k) => k.encrypt(rng, plain),
        }
    }

    fn to_writer_old(&self, writer: &mut impl io::Write) -> native::errors::Result<()> {
        match self {
            Self::Key(k) => k.to_writer_old(writer),
            Self::Subkey(k) => k.to_writer_old(writer),
        }
    }
}

/// Find primary key or subkey to use for encryption.
///
/// First, tries to use subkeys. If none of the subkeys are suitable
/// for encryption, tries to use primary key. Returns `None` if the
/// public key cannot be used for encryption.
fn find_pkey_for_encryption(key: &SignedPublicKey) -> Option<SignedPublicKeyOrSubkey<'_>> {
    if key.is_encryption_key() {
        Some(SignedPublicKeyOrSubkey::Key(key))
    } else {
        key.public_subkeys
            .iter()
            .find(|subkey| subkey.is_encryption_key())
            .map(SignedPublicKeyOrSubkey::Subkey)
    }
}

/// Encrypts given bytes using the given list of public keys.
pub async fn encrypt(pkeys: Vec<SignedPublicKey>, plain_bytes: Vec<u8>) -> Result<Vec<u8>> {
    spawn_blocking(move || {
        let mut rng = thread_rng();

        let msg = Message::new_literal_bytes("", &plain_bytes);

        let pkeys: Vec<SignedPublicKeyOrSubkey> =
            pkeys.iter().filter_map(find_pkey_for_encryption).collect();
        let pkeys_refs: Vec<&SignedPublicKeyOrSubkey> = pkeys.iter().collect();

        let encrypted_bytes = msg
            .compress(CompressionAlgorithm::ZLIB)
            .map_err(Error::CompressMessageError)?
            .encrypt_to_keys(&mut rng, Default::default(), &pkeys_refs)
            .map_err(Error::EncryptMessageError)?
            .to_armored_bytes(None)
            .map_err(Error::ExportEncryptedMessageToArmorError)?;

        Ok(encrypted_bytes)
    })
    .await?
}
