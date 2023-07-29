#![doc = include_str!("../README.md")]

pub mod mml;
pub mod pgp;
pub mod tpl;

pub use self::{
    mml::FilterParts,
    pgp::{
        PgpPublicKey, PgpPublicKeyResolver, PgpPublicKeys, PgpPublicKeysResolver, PgpSecretKey,
        PgpSecretKeyResolver,
    },
    tpl::{ShowHeadersStrategy, Tpl, TplInterpreter},
};

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    TplCompilerError(#[from] tpl::Error),
    #[error(transparent)]
    MmlCompilerError(#[from] mml::compiler::Error),
    #[error(transparent)]
    PgpError(#[from] pimalaya_pgp::Error),
    #[error(transparent)]
    KeyringError(#[from] pimalaya_keyring::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
