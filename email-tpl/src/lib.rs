#![doc = include_str!("../README.md")]

pub mod mml;
pub mod pgp;
pub mod tpl;

pub use self::{
    mml::FilterParts,
    pgp::{Gpg, Pgp, PgpNative, PgpNativePublicKeysResolver, PgpNativeSecretKey, SignedSecretKey},
    tpl::{ShowHeadersStrategy, Tpl, TplInterpreter},
};

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    TplError(#[from] tpl::Error),
    #[error(transparent)]
    CompileMmlError(#[from] mml::compiler::Error),
    #[error(transparent)]
    InterpretTplError(#[from] tpl::interpreter::Error),
    #[error(transparent)]
    InterpretMmlError(#[from] mml::interpreter::Error),
    #[error(transparent)]
    PgpError(#[from] pgp::Error),
    #[error(transparent)]
    KeyringError(#[from] pimalaya_keyring::Error),

    #[error(transparent)]
    PimalayaPgpError(#[from] pimalaya_pgp::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
