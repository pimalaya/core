#[cfg(feature = "key-discovery")]
use hyper::Uri;
use pgp_native::{SecretKeyParamsBuilderError, SubkeyParamsBuilderError};
use std::{
    io,
    path::{self, PathBuf},
    result,
};
use thiserror::Error;
use tokio::task::JoinError;

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot import armored pgp message")]
    ImportMessageFromArmorError(#[source] pgp_native::errors::Error),
    #[error("cannot decrypt pgp message")]
    DecryptMessageError(#[source] pgp_native::errors::Error),
    #[error("cannot decompress pgp message")]
    DecompressMessageError(#[source] pgp_native::errors::Error),
    #[error("cannot get pgp message content")]
    GetMessageContentError(#[source] pgp_native::errors::Error),
    #[error("cannot get empty pgp message content")]
    GetMessageContentEmptyError,
    #[error("cannot get empty pgp message")]
    GetMessageEmptyError,

    #[error("cannot find pgp secret key for signing message")]
    FindSignedSecretKeyForSigningError,
    #[error("cannot sign pgp message")]
    SignMessageError(#[source] pgp_native::errors::Error),
    #[error("cannot export signed pgp message as armored string")]
    ExportSignedMessageToArmoredBytesError(#[source] pgp_native::errors::Error),
    #[error("cannot encrypt message using pgp")]
    EncryptMessageError(#[source] pgp_native::errors::Error),
    #[error("cannot export encrypted pgp message as armored string")]
    ExportEncryptedMessageToArmorError(#[source] pgp_native::errors::Error),
    #[error("cannot compress pgp message")]
    CompressMessageError(#[source] pgp_native::errors::Error),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse body from {1}")]
    ParseBodyWithUriError(#[source] hyper::Error, Uri),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse response from {1}")]
    FetchResponseError(#[source] hyper::Error, Uri),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse pgp public key from {1}")]
    ParsePublicKeyError(#[source] pgp_native::errors::Error, Uri),
    #[error("cannot find pgp public key for email {0}")]
    FindPublicKeyError(String),
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

    #[error("cannot verify pgp signature")]
    VerifySignatureError(#[source] pgp_native::errors::Error),
    #[error("cannot parse email address {0}")]
    ParseEmailAddressError(String),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse url {1}")]
    ParseUrlError(#[source] url::ParseError, String),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse uri {1}")]
    ParseUriError(#[source] hyper::http::uri::InvalidUri, String),
    #[error("cannot parse path {1}")]
    ParseFilePathError(path::StripPrefixError, url::Url),
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse response")]
    ParseResponseError(#[source] hyper::Error),
    #[error("cannot parse response: too many redirect")]
    RedirectOverflowError,
    #[cfg(feature = "key-discovery")]
    #[error("cannot parse body")]
    ParseBodyError(#[source] hyper::Error),
    #[error("cannot parse certificate")]
    ParseCertError(#[source] pgp_native::errors::Error),

    #[error(transparent)]
    JoinError(#[from] JoinError),
}
