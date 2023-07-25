pub mod config;
pub mod decrypt;
pub mod encrypt;
pub mod hkps;
pub mod sign;
pub mod verify;
pub mod wkd;

pub use config::{generate_key_pair, read_armored_public_key, read_armored_secret_key};
pub use decrypt::decrypt;
pub use encrypt::encrypt;
pub use sign::sign;
pub use verify::verify;

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    ConfigError(#[from] config::Error),

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
    HkpsError(#[from] hkps::Error),

    #[error("cannot perform pgp action: pgp not configured")]
    None,
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
