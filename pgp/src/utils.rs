//! Module dedicated to PGP helpers.

use pgp_native::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::{CompressionAlgorithm, SecretKeyTrait},
    Deserializable, KeyType, SecretKeyParamsBuilder, SecretKeyParamsBuilderError, SignedPublicKey,
    SignedSecretKey, StandaloneSignature, SubkeyParamsBuilder, SubkeyParamsBuilderError,
};
use smallvec::smallvec;
use std::{
    fs,
    io::{self, Cursor},
    path::PathBuf,
};
use thiserror::Error;
use tokio::task;

use crate::Result;

/// Errors related to PGP helpers.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build pgp secret key params")]
    BuildSecretKeyParamsError(#[source] SecretKeyParamsBuilderError),
    #[error("cannot generate pgp secret key")]
    GenerateSecretKeyError(#[source] pgp_native::errors::Error),
    #[error("cannot sign pgp secret key")]
    SignSecretKeyError(#[source] pgp_native::errors::Error),
    #[error("cannot verify pgp secret key")]
    VerifySecretKeyError(#[source] pgp_native::errors::Error),

    #[error("cannot build pgp public subkey params")]
    BuildPublicKeyParamsError(#[source] SubkeyParamsBuilderError),
    #[error("cannot sign pgp public subkey")]
    SignPublicKeyError(#[source] pgp_native::errors::Error),
    #[error("cannot verify pgp public subkey")]
    VerifyPublicKeyError(#[source] pgp_native::errors::Error),

    #[error("cannot read armored public key at {1}")]
    ReadArmoredPublicKeyError(#[source] io::Error, PathBuf),
    #[error("cannot parse armored public key from {1}")]
    ParseArmoredPublicKeyError(#[source] pgp_native::errors::Error, PathBuf),

    #[error("cannot read armored secret key file {1}")]
    ReadArmoredSecretKeyFromPathError(#[source] io::Error, PathBuf),
    #[error("cannot parse armored secret key from {1}")]
    ParseArmoredSecretKeyFromPathError(#[source] pgp_native::errors::Error, PathBuf),
    #[error("cannot parse armored secret key from string")]
    ParseArmoredSecretKeyFromStringError(#[source] pgp_native::errors::Error),

    #[error("cannot import pgp signature from armor")]
    ReadStandaloneSignatureFromArmoredBytesError(#[source] pgp_native::errors::Error),
}

/// Generates a new pair of secret and public keys for the given email
/// address and passphrase.
pub async fn gen_key_pair(
    email: impl ToString,
    passphrase: impl ToString,
) -> Result<(SignedSecretKey, SignedPublicKey)> {
    let email = email.to_string();
    let passphrase = passphrase.to_string();
    let passphrase = if passphrase.trim().is_empty() {
        None
    } else {
        Some(passphrase)
    };

    task::spawn_blocking(move || {
        let key_params = SecretKeyParamsBuilder::default()
            .key_type(KeyType::EdDSA)
            .can_create_certificates(true)
            .can_sign(true)
            .primary_user_id(email)
            .passphrase(passphrase.clone())
            .preferred_symmetric_algorithms(smallvec![SymmetricKeyAlgorithm::AES256])
            .preferred_hash_algorithms(smallvec![HashAlgorithm::SHA2_256])
            .preferred_compression_algorithms(smallvec![CompressionAlgorithm::ZLIB])
            .subkey(
                SubkeyParamsBuilder::default()
                    .key_type(KeyType::ECDH)
                    .can_encrypt(true)
                    .passphrase(passphrase)
                    .build()
                    .map_err(Error::BuildPublicKeyParamsError)?,
            )
            .build()
            .map_err(Error::BuildSecretKeyParamsError)?;

        let skey = key_params
            .generate()
            .map_err(Error::GenerateSecretKeyError)?;
        let skey = skey.sign(String::new).map_err(Error::SignSecretKeyError)?;
        skey.verify().map_err(Error::VerifySecretKeyError)?;

        let pkey = skey.public_key();
        let pkey = pkey
            .sign(&skey, String::new)
            .map_err(Error::SignPublicKeyError)?;
        pkey.verify().map_err(Error::VerifyPublicKeyError)?;

        Ok((skey, pkey))
    })
    .await?
}

/// Reads a signed public key from the given path.
///
/// The given path needs to contain a single armored secret key,
/// otherwise it fails.
pub async fn read_pkey_from_path(path: PathBuf) -> Result<SignedPublicKey> {
    task::spawn_blocking(move || {
        let data =
            fs::read(&path).map_err(|err| Error::ReadArmoredPublicKeyError(err, path.clone()))?;
        let (pkey, _) = SignedPublicKey::from_armor_single(Cursor::new(data))
            .map_err(|err| Error::ParseArmoredPublicKeyError(err, path.clone()))?;
        Ok(pkey)
    })
    .await?
}

/// Reads a signed secret key from the given path.
///
/// The given path needs to contain a single armored secret key,
/// otherwise it fails.
pub async fn read_skey_from_file(path: PathBuf) -> Result<SignedSecretKey> {
    task::spawn_blocking(move || {
        let data = fs::read(&path)
            .map_err(|err| Error::ReadArmoredSecretKeyFromPathError(err, path.clone()))?;
        let (skey, _) = SignedSecretKey::from_armor_single(Cursor::new(data))
            .map_err(|err| Error::ParseArmoredSecretKeyFromPathError(err, path.clone()))?;
        Ok(skey)
    })
    .await?
}

/// Reads a signed secret key from the given raw string.
///
/// The given raw string needs to contain a single armored secret key,
/// otherwise it fails.
pub async fn read_skey_from_string(string: String) -> Result<SignedSecretKey> {
    task::spawn_blocking(move || {
        let (skey, _) = SignedSecretKey::from_armor_single(Cursor::new(string))
            .map_err(Error::ParseArmoredSecretKeyFromStringError)?;
        Ok(skey)
    })
    .await?
}

/// Reads a standalone signature from the given raw bytes.
///
/// The given raw bytes needs to match a single armored signature,
/// otherwise it fails.
pub async fn read_sig_from_bytes(bytes: Vec<u8>) -> Result<StandaloneSignature> {
    task::spawn_blocking(move || {
        let (sig, _) = StandaloneSignature::from_armor_single(Cursor::new(&bytes))
            .map_err(Error::ReadStandaloneSignatureFromArmoredBytesError)?;
        Ok(sig)
    })
    .await?
}
