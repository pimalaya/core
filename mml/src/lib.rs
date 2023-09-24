//! # 📫 MIME Meta Language
//!
//! Rust implementation of the Emacs MIME message Meta Language, as known as [MML].
//!
//! This library exposes a [MML to MIME](MmlCompilerBuilder) message
//! compiler and a [MIME to MML](MimeInterpreterBuilder) message
//! interpreter.
//!
//! ```eml,ignore
#![doc = include_str!("../examples/plain.eml")]
//! ```
//!
//! ```rust,ignore
#![doc = include_str!("../examples/plain.rs")]
//! ```
//!
//! [MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html

pub mod message;
#[cfg(feature = "pgp")]
pub mod pgp;

#[cfg(feature = "compiler")]
#[doc(inline)]
pub use self::message::{CompileMmlResult, MmlCompiler, MmlCompilerBuilder};
#[cfg(feature = "interpreter")]
#[doc(inline)]
pub use self::message::{MimeInterpreter, MimeInterpreterBuilder};

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
    CmdsPgpError(#[from] pgp::commands::Error),

    #[cfg(feature = "pgp-native")]
    #[error(transparent)]
    NativePgpError(#[from] pgp::native::Error),

    #[cfg(feature = "pgp-gpg")]
    #[error(transparent)]
    GpgError(#[from] pgp::gpg::Error),
}

/// The global `Result` alias of the library.
pub type Result<T> = std::result::Result<T, Error>;
