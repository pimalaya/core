//! # Error
//!
//! Module dedicated to pgp errors. It contains an [`Error`] enum
//! based on [`thiserror::Error`] and a type alias [`Result`].

use std::path::PathBuf;

use thiserror::Error;

use crate::native::{self, SecretKeyParamsBuilderError, SubkeyParamsBuilderError};

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot import armored pgp message")]
    ImportMessageFromArmorError(#[source] native::errors::Error),
    #[error("cannot decrypt pgp message")]
    DecryptMessageError(#[source] native::errors::Error),
    #[error("cannot decompress pgp message")]
    DecompressMessageError(#[source] native::errors::Error),
    #[error("cannot get pgp message content")]
    GetMessageContentError(#[source] native::errors::Error),
    #[error("cannot get empty pgp message content")]
    GetMessageContentEmptyError,
    #[error("cannot get empty pgp message")]
    GetMessageEmptyError,

    #[error("cannot find pgp secret key for signing message")]
    FindSignedSecretKeyForSigningError,
    #[error("cannot sign pgp message")]
    SignMessageError(#[source] native::errors::Error),
    #[error("cannot export signed pgp message as armored string")]
    ExportSignedMessageToArmoredBytesError(#[source] native::errors::Error),
    #[error("cannot encrypt message using pgp")]
    EncryptMessageError(#[source] native::errors::Error),
    #[error("cannot export encrypted pgp message as armored string")]
    ExportEncryptedMessageToArmorError(#[source] native::errors::Error),
    #[error("cannot compress pgp message")]
    CompressMessageError(#[source] native::errors::Error),
    #[cfg(feature = "key-discovery")]
    #[error("cannot get public key at {1}: {2}: {0}")]
    GetPublicKeyError(String, http::ureq::http::Uri, http::ureq::http::StatusCode),
    #[cfg(feature = "key-discovery")]
    #[error("cannot read HTTP error from {1}: {2}")]
    ReadHttpError(
        #[source] std::io::Error,
        http::ureq::http::Uri,
        http::ureq::http::StatusCode,
    ),
    #[cfg(feature = "key-discovery")]
    #[error("cannot read PGP public key from {1}")]
    ReadPublicKeyError(#[source] std::io::Error, http::ureq::http::Uri),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse PGP armored public key from {1}")]
    ParsePublicKeyError(#[source] native::errors::Error, http::ureq::http::Uri),
    #[cfg(feature = "key-discovery")]
    #[error(transparent)]
    HttpError(#[from] http::Error),
    #[cfg(feature = "key-discovery")]
    #[error("cannot find pgp public key for email {0}")]
    FindPublicKeyError(String),
    #[error("cannot build pgp secret key params")]
    BuildSecretKeyParamsError(#[source] SecretKeyParamsBuilderError),
    #[error("cannot generate pgp secret key")]
    GenerateSecretKeyError(#[source] native::errors::Error),
    #[error("cannot sign pgp secret key")]
    SignSecretKeyError(#[source] native::errors::Error),
    #[error("cannot verify pgp secret key")]
    VerifySecretKeyError(#[source] native::errors::Error),

    #[error("cannot build pgp public subkey params")]
    BuildPublicKeyParamsError(#[source] SubkeyParamsBuilderError),
    #[error("cannot sign pgp public subkey")]
    SignPublicKeyError(#[source] native::errors::Error),
    #[error("cannot verify pgp public subkey")]
    VerifyPublicKeyError(#[source] native::errors::Error),

    #[error("cannot read armored public key at {1}")]
    ReadArmoredPublicKeyError(#[source] std::io::Error, PathBuf),
    #[error("cannot parse armored public key from {1}")]
    ParseArmoredPublicKeyError(#[source] native::errors::Error, PathBuf),

    #[error("cannot read armored secret key file {1}")]
    ReadArmoredSecretKeyFromPathError(#[source] std::io::Error, PathBuf),
    #[error("cannot parse armored secret key from {1}")]
    ParseArmoredSecretKeyFromPathError(#[source] native::errors::Error, PathBuf),
    #[error("cannot parse armored secret key from string")]
    ParseArmoredSecretKeyFromStringError(#[source] native::errors::Error),
    #[error("cannot parse armored secret key from string")]
    ParseArmoredPublicKeyFromStringError(#[source] native::errors::Error),

    #[error("cannot import pgp signature from armor")]
    ReadStandaloneSignatureFromArmoredBytesError(#[source] native::errors::Error),

    #[error("cannot verify pgp signature")]
    VerifySignatureError(#[source] native::errors::Error),
    #[error("cannot parse email address {0}")]
    ParseEmailAddressError(String),
    #[cfg(feature = "key-discovery")]
    #[error("cannot create HTTP connector")]
    CreateHttpConnectorError(#[source] std::io::Error),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse uri {1}")]
    ParseUriError(#[source] http::Error, String),
    #[cfg(feature = "key-discovery")]
    #[error("cannot build key server URI from {1}")]
    BuildKeyServerUriError(#[source] http::Error, http::ureq::http::Uri),
    #[error("cannot parse response: too many redirect")]
    RedirectOverflowError,
    #[error("cannot parse certificate")]
    ParseCertError(#[source] native::errors::Error),

    #[cfg(feature = "tokio")]
    #[error(transparent)]
    JoinError(#[from] tokio::task::JoinError),
}
