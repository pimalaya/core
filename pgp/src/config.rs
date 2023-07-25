use pgp::{
    crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
    types::{CompressionAlgorithm, SecretKeyTrait},
    Deserializable, KeyType, SecretKeyParamsBuilder, SecretKeyParamsBuilderError, SignedPublicKey,
    SignedSecretKey, SubkeyParamsBuilder, SubkeyParamsBuilderError,
};
use smallvec::smallvec;
use std::{
    fs,
    io::{self, Cursor},
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::Result;

/// Errors related to configuration.
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

    #[error("cannot build pgp public subkey params")]
    BuildPublicKeyParamsError(#[from] SubkeyParamsBuilderError),
    #[error("cannot sign pgp public subkey")]
    SignPublicKeyError(#[source] pgp::errors::Error),
    #[error("cannot verify pgp public subkey")]
    VerifyPublicKeyError(#[source] pgp::errors::Error),

    #[error("cannot read armored public key at {1}")]
    ReadArmoredPublicKeyError(io::Error, PathBuf),
    #[error("cannot parse armored public key from {1}")]
    ParseArmoredPublicKeyError(pgp::errors::Error, PathBuf),

    #[error("cannot read armored secret key at {1}")]
    ReadArmoredSecretKeyError(io::Error, PathBuf),
    #[error("cannot parse armored secret key from {1}")]
    ParseArmoredSecretKeyError(pgp::errors::Error, PathBuf),
}

/// Generates a new pair of private/public keys for the given email
/// address.
pub fn generate_key_pair(email: impl ToString) -> Result<(SignedSecretKey, SignedPublicKey)> {
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
                .map_err(Error::BuildPublicKeyParamsError)?,
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

/// Reads a signed public key from the given path.
pub fn read_armored_public_key(path: impl AsRef<Path>) -> Result<SignedPublicKey> {
    let path = path.as_ref();
    let input =
        fs::read(path).map_err(|err| Error::ReadArmoredPublicKeyError(err, path.to_owned()))?;
    let cursor = Cursor::new(input);
    let (pkey, _) = SignedPublicKey::from_armor_single(cursor)
        .map_err(|err| Error::ParseArmoredPublicKeyError(err, path.to_owned()))?;
    Ok(pkey)
}

/// Reads a signed secret key from the given path.
pub fn read_armored_secret_key(path: impl AsRef<Path>) -> Result<SignedSecretKey> {
    let path = path.as_ref();
    let input =
        fs::read(path).map_err(|err| Error::ReadArmoredSecretKeyError(err, path.to_owned()))?;
    let cursor = Cursor::new(input);
    let (skey, _) = SignedSecretKey::from_armor_single(cursor)
        .map_err(|err| Error::ParseArmoredSecretKeyError(err, path.to_owned()))?;
    Ok(skey)
}
