#![doc = include_str!("../README.md")]

pub mod body;
pub(crate) mod header;
pub mod message;
#[cfg(feature = "pgp")]
pub mod pgp;

#[cfg(feature = "pgp-cmds")]
pub use self::pgp::CmdsPgp;
#[cfg(feature = "pgp-gpg")]
pub use self::pgp::Gpg;
#[cfg(feature = "pgp")]
pub use self::pgp::Pgp;
#[cfg(feature = "pgp-native")]
pub use self::pgp::{
    NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, SignedPublicKey, SignedSecretKey,
};
#[cfg(feature = "compiler")]
pub use self::{body::MmlBodyCompiler, message::MmlCompiler};
#[cfg(feature = "interpreter")]
pub use self::{
    body::{FilterParts, MmlBodyInterpreter},
    message::{MmlInterpreter, ShowHeadersStrategy},
};

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "compiler")]
    #[error(transparent)]
    CompileMmlError(#[from] message::compiler::Error),

    #[cfg(feature = "compiler")]
    #[error(transparent)]
    CompileMmlBodyError(#[from] body::compiler::Error),

    #[cfg(feature = "interpreter")]
    #[error(transparent)]
    InterpretMmlError(#[from] message::interpreter::Error),

    #[cfg(feature = "interpreter")]
    #[error(transparent)]
    InterpretMmlBodyError(#[from] body::interpreter::Error),

    #[cfg(feature = "pgp")]
    #[error(transparent)]
    PgpError(#[from] pgp::Error),

    #[cfg(feature = "pgp-cmds")]
    #[error(transparent)]
    CmdsPgpError(#[from] pgp::cmds::Error),

    #[cfg(feature = "pgp-native")]
    #[error(transparent)]
    NativePgpError(#[from] pgp::native::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error(transparent)]
    GpgError(#[from] pgp::gpg::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
