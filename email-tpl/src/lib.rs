#![doc = include_str!("../README.md")]

pub mod mml;
pub mod pgp;
pub mod tpl;

#[cfg(feature = "cmds-pgp")]
pub use self::pgp::CmdsPgp;
#[cfg(feature = "gpg")]
pub use self::pgp::Gpg;
#[cfg(feature = "native-pgp")]
pub use self::pgp::{
    NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, SignedPublicKey, SignedSecretKey,
};
pub use self::{
    mml::FilterParts,
    pgp::Pgp,
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
    // #[error(transparent)]
    // KeyringError(#[from] pimalaya_keyring::Error),
    #[error(transparent)]
    PgpError(#[from] pgp::Error),
    #[cfg(feature = "cmds-pgp")]
    #[error(transparent)]
    CmdsPgpError(#[from] pgp::cmds::Error),
    #[cfg(feature = "native-pgp")]
    #[error(transparent)]
    NativePgpError(#[from] pgp::native::Error),
    #[cfg(feature = "gpg")]
    #[error(transparent)]
    GpgError(#[from] pgp::gpg::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
