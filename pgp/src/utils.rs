//! # Utils
//!
//! Module dedicated to PGP helpers.

use std::{fs, io::Cursor, path::PathBuf};

use smallvec::smallvec;

use crate::{
    native::{
        crypto::{hash::HashAlgorithm, sym::SymmetricKeyAlgorithm},
        types::{CompressionAlgorithm, SecretKeyTrait},
        Deserializable, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SignedSecretKey,
        StandaloneSignature, SubkeyParamsBuilder,
    },
    Error, Result,
};

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

    spawn_blocking(move || {
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
    spawn_blocking(move || {
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
    spawn_blocking(move || {
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
    spawn_blocking(move || {
        let (skey, _) = SignedSecretKey::from_armor_single(Cursor::new(string))
            .map_err(Error::ParseArmoredSecretKeyFromStringError)?;
        Ok(skey)
    })
    .await?
}

/// Reads a signed public key from the given raw string.
///
/// The given raw string needs to contain a single armored secret key,
/// otherwise it fails.
pub async fn read_pkey_from_string(string: String) -> Result<SignedPublicKey> {
    spawn_blocking(move || {
        let (skey, _) = SignedPublicKey::from_armor_single(Cursor::new(string))
            .map_err(Error::ParseArmoredPublicKeyFromStringError)?;
        Ok(skey)
    })
    .await?
}

/// Reads a standalone signature from the given raw bytes.
///
/// The given raw bytes needs to match a single armored signature,
/// otherwise it fails.
pub async fn read_sig_from_bytes(bytes: Vec<u8>) -> Result<StandaloneSignature> {
    spawn_blocking(move || {
        let (sig, _) = StandaloneSignature::from_armor_single(Cursor::new(&bytes))
            .map_err(Error::ReadStandaloneSignatureFromArmoredBytesError)?;
        Ok(sig)
    })
    .await?
}

#[cfg(feature = "key-discovery")]
#[cfg(feature = "async-std")]
pub(crate) async fn spawn<F>(f: F) -> Result<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    Ok(async_std::task::spawn(f).await)
}

#[cfg(feature = "async-std")]
pub(crate) async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(async_std::task::spawn_blocking(f).await)
}

#[cfg(feature = "key-discovery")]
#[cfg(feature = "tokio")]
pub(crate) async fn spawn<F>(f: F) -> Result<F::Output>
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    Ok(tokio::task::spawn(f).await?)
}

#[cfg(feature = "tokio")]
pub(crate) async fn spawn_blocking<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    Ok(tokio::task::spawn_blocking(f).await?)
}
