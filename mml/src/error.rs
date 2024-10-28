use std::{io, path::PathBuf};

use thiserror::Error;

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum Error {
    #[error("missing PGP configuration")]
    PgpMissingConfigurationError,

    #[cfg(feature = "compiler")]
    #[error("cannot parse MML body")]
    ParseMmlError(Vec<chumsky::error::Rich<'static, char>>, String),
    #[cfg(feature = "compiler")]
    #[error("cannot compile template")]
    WriteCompiledPartToVecError(#[source] io::Error),
    #[cfg(feature = "compiler")]
    #[error("cannot read attachment at {1:?}")]
    ReadAttachmentError(#[source] io::Error, PathBuf),

    #[cfg(feature = "pgp")]
    #[error("cannot sign part using pgp: missing sender")]
    PgpSignMissingSenderError,

    #[cfg(all(feature = "pgp-native", feature = "keyring"))]
    #[error("cannot get pgp secret key from keyring")]
    GetSecretKeyFromKeyringError(#[source] secret::keyring::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot read pgp secret key from keyring")]
    ReadSecretKeyFromKeyringError(pgp::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot read pgp secret key from path {1}")]
    ReadSecretKeyFromPathError(pgp::Error, PathBuf),

    #[cfg(feature = "pgp-native")]
    #[error("cannot get pgp secret key passphrase from keyring")]
    GetSecretKeyPassphraseFromKeyringError(#[source] secret::Error),

    #[cfg(all(feature = "pgp-native", feature = "keyring"))]
    #[error("cannot get pgp secret key from keyring")]
    GetPgpSecretKeyFromKeyringError(#[source] secret::keyring::Error),

    #[error("cannot get native pgp secret key of {0}")]
    GetNativePgpSecretKeyNoneError(String),
    #[error("cannot find native pgp public key of {0}")]
    FindPgpPublicKeyError(String),

    #[cfg(feature = "pgp-native")]
    #[error("cannot encrypt data using native pgp")]
    EncryptNativePgpError(#[source] pgp::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot decrypt data using native pgp")]
    DecryptNativePgpError(#[source] pgp::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot sign data using native pgp")]
    SignNativePgpError(#[source] pgp::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot read native pgp signature")]
    ReadNativePgpSignatureError(#[source] pgp::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot verify native pgp signature")]
    VerifyNativePgpSignatureError(#[source] pgp::Error),

    #[cfg(feature = "pgp-native")]
    #[error("cannot read native pgp secret key")]
    ReadNativePgpSecretKeyError(#[source] pgp::Error),

    #[error("cannot parse MIME message")]
    ParseMimeMessageError,
    #[error("cannot save attachment at {1}")]
    WriteAttachmentError(#[source] io::Error, PathBuf),
    #[error("cannot build email")]
    WriteMessageError(#[source] io::Error),
    #[error("cannot parse pgp decrypted part")]
    ParsePgpDecryptedPartError,
    #[error("cannot decrypt part using pgp: missing recipient")]
    PgpDecryptMissingRecipientError,

    #[error("cannot parse template")]
    ParseMessageError,
    #[error("cannot parse MML message: empty body")]
    ParseMmlEmptyBodyError,
    #[error("cannot parse MML message: empty body content")]
    ParseMmlEmptyBodyContentError,
    #[error("cannot compile MML message to vec")]
    CompileMmlMessageToVecError(#[source] io::Error),
    #[error("cannot compile MML message to string")]
    CompileMmlMessageToStringError(#[source] io::Error),

    #[error("cannot parse raw email")]
    ParseRawEmailError,
    #[error("cannot build email")]
    BuildEmailError(#[source] io::Error),

    #[cfg(feature = "pgp-commands")]
    #[error("cannot encrypt data using commands")]
    EncryptCommandError(#[source] process::Error),

    #[cfg(feature = "pgp-commands")]
    #[error("cannot decrypt data using commands")]
    DecryptCommandError(#[source] process::Error),

    #[cfg(feature = "pgp-commands")]
    #[error("cannot sign data using commands")]
    SignCommandError(#[source] process::Error),

    #[cfg(feature = "pgp-commands")]
    #[error("cannot verify data using commands")]
    VerifyCommandError(#[source] process::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot get gpg context")]
    GetContextError(#[source] gpgme::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot get gpg home dir path from {0}")]
    GetHomeDirPathError(PathBuf),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot set gpg home dir at {1}")]
    SetHomeDirError(#[source] gpgme::Error, PathBuf),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot encrypt data using gpg")]
    EncryptGpgError(#[source] gpgme::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot decrypt data using gpg")]
    DecryptGpgError(#[source] gpgme::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot sign data using gpg")]
    SignGpgError(#[source] gpgme::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error("cannot verify data using gpg")]
    VerifyGpgError(#[source] gpgme::Error),
}
