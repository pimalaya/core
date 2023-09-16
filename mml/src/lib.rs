#![doc = include_str!("../README.md")]

pub mod message;
#[cfg(feature = "pgp")]
pub mod pgp;

#[cfg(feature = "interpreter")]
pub use self::message::{FilterParts, MimeBodyInterpreter, MimeInterpreter, ShowHeadersStrategy};
#[cfg(feature = "compiler")]
pub use self::message::{MmlBodyCompiler, MmlCompiler};
#[cfg(feature = "pgp-commands")]
pub use self::pgp::CmdsPgp;
#[cfg(feature = "pgp-gpg")]
pub use self::pgp::Gpg;
#[cfg(feature = "pgp")]
pub use self::pgp::Pgp;
#[cfg(feature = "pgp-native")]
pub use self::pgp::{
    NativePgp, NativePgpPublicKeysResolver, NativePgpSecretKey, SignedPublicKey, SignedSecretKey,
};

/// The global `Error` enum of the library.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[cfg(feature = "compiler")]
    #[error(transparent)]
    CompileMmlError(#[from] message::compiler::Error),

    #[cfg(feature = "compiler")]
    #[error(transparent)]
    CompileMmlBodyError(#[from] message::body::compiler::Error),

    #[cfg(feature = "interpreter")]
    #[error(transparent)]
    InterpretMimeError(#[from] message::interpreter::Error),

    #[cfg(feature = "interpreter")]
    #[error(transparent)]
    InterpretMimeBodyError(#[from] message::body::interpreter::Error),

    #[cfg(feature = "pgp")]
    #[error(transparent)]
    PgpError(#[from] pgp::Error),

    #[cfg(feature = "pgp-commands")]
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
