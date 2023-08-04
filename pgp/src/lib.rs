pub mod decrypt;
pub mod encrypt;
pub mod hkp;
pub mod http;
pub mod sign;
pub mod utils;
pub mod verify;
pub mod wkd;

pub(crate) mod client;

#[doc(inline)]
pub use pgp::{errors::Error as NativeError, SignedPublicKey, SignedSecretKey};
use tokio::task::JoinError;

#[doc(inline)]
pub use self::{
    decrypt::decrypt,
    encrypt::encrypt,
    sign::sign,
    utils::{
        gen_key_pair, read_sig_from_bytes, read_signed_public_key_from_path, read_skey_from_file,
        read_skey_from_string,
    },
    verify::verify,
};

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ConfigError(#[from] utils::Error),

    #[error(transparent)]
    EncryptError(#[from] encrypt::Error),
    #[error(transparent)]
    DecryptError(#[from] decrypt::Error),
    #[error(transparent)]
    SignError(#[from] sign::Error),
    #[error(transparent)]
    VerifyError(#[from] verify::Error),

    #[error(transparent)]
    WkdError(#[from] wkd::Error),
    #[error(transparent)]
    HttpError(#[from] http::Error),

    #[error(transparent)]
    JoinError(#[from] JoinError),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
