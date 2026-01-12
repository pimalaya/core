//! # Sign
//!
//! Module dedicated to PGP signing. This module exposes a simple
//! function [`sign`] and its associated [`Error`]s.

use std::io;

use rand::{CryptoRng, Rng};

use crate::{
    native::{
        self,
        crypto::{hash::HashAlgorithm, public_key::PublicKeyAlgorithm},
        types::{KeyId, KeyTrait, Mpi, PublicKeyTrait, SecretKeyRepr, SecretKeyTrait},
        Message, PublicKey, PublicSubkey, SignedSecretKey, SignedSecretSubKey,
    },
    utils::spawn_blocking,
    Error, Result,
};

#[derive(Debug)]
pub enum PublicKeyOrSubkey {
    Key(PublicKey),
    Subkey(PublicSubkey),
}

#[derive(Debug)]
pub enum SignedSecretKeyOrSubkey<'a> {
    Key(&'a SignedSecretKey),
    Subkey(&'a SignedSecretSubKey),
}

impl KeyTrait for SignedSecretKeyOrSubkey<'_> {
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

impl PublicKeyTrait for SignedSecretKeyOrSubkey<'_> {
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

    fn encrypt<R: CryptoRng + Rng>(
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

impl<'a> SecretKeyTrait for SignedSecretKeyOrSubkey<'a> {
    type PublicKey = PublicKeyOrSubkey;

    fn unlock<F, G>(&self, pw: F, work: G) -> native::errors::Result<()>
    where
        F: FnOnce() -> String,
        G: FnOnce(&SecretKeyRepr) -> native::errors::Result<()>,
    {
        match self {
            Self::Key(k) => k.unlock(pw, work),
            Self::Subkey(k) => k.unlock(pw, work),
        }
    }

    fn create_signature<F>(
        &self,
        key_pw: F,
        hash: HashAlgorithm,
        data: &[u8],
    ) -> native::errors::Result<Vec<Mpi>>
    where
        F: FnOnce() -> String,
    {
        match self {
            Self::Key(k) => k.create_signature(key_pw, hash, data),
            Self::Subkey(k) => k.create_signature(key_pw, hash, data),
        }
    }

    fn public_key(&self) -> Self::PublicKey {
        match self {
            Self::Key(k) => PublicKeyOrSubkey::Key(k.public_key()),
            Self::Subkey(k) => PublicKeyOrSubkey::Subkey(k.public_key()),
        }
    }
}

/// Find primary key or subkey to use for signing.
///
/// First, tries to use subkeys. If none of the subkeys are suitable
/// for signing, tries to use primary key. Returns `None` if the
/// public key cannot be used for signing.
fn find_skey_for_signing(key: &SignedSecretKey) -> Option<SignedSecretKeyOrSubkey<'_>> {
    if key.is_signing_key() {
        Some(SignedSecretKeyOrSubkey::Key(key))
    } else {
        key.secret_subkeys
            .iter()
            .find(|subkey| subkey.is_signing_key())
            .map(SignedSecretKeyOrSubkey::Subkey)
    }
}

/// Signs given bytes using the given private key and its passphrase.
pub async fn sign(
    skey: SignedSecretKey,
    passphrase: impl ToString,
    plain_bytes: Vec<u8>,
) -> Result<Vec<u8>> {
    let passphrase = passphrase.to_string();

    spawn_blocking(move || {
        let skey = find_skey_for_signing(&skey).ok_or(Error::FindSignedSecretKeyForSigningError)?;

        let msg = Message::new_literal_bytes("", &plain_bytes)
            .sign(&skey, || passphrase, HashAlgorithm::SHA2_256)
            .map_err(Error::SignMessageError)?;

        let signature_bytes = msg
            .into_signature()
            .to_armored_bytes(None)
            .map_err(Error::ExportSignedMessageToArmoredBytesError)?;

        Ok(signature_bytes)
    })
    .await?
}
