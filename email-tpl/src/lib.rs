#![doc = include_str!("../README.md")]

pub mod crypto;
pub mod mml;
pub mod tpl;

pub use self::{
    crypto::{Decrypt, Encrypt, PgpDecrypt, PgpEncrypt, PgpSign, PgpVerify, Sign, Verify},
    mml::FilterParts,
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
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
