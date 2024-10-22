#![cfg_attr(docs_rs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

mod error;
pub mod message;
#[cfg(feature = "pgp")]
pub mod pgp;

#[doc(inline)]
pub use crate::error::{Error, Result};
#[cfg(feature = "interpreter")]
#[doc(inline)]
pub use crate::message::{MimeInterpreter, MimeInterpreterBuilder};
#[cfg(feature = "compiler")]
#[doc(inline)]
pub use crate::message::{MmlCompileResult, MmlCompiler, MmlCompilerBuilder};

#[cfg(any(feature = "pgp-commands", feature = "pgp-native"))]
#[cfg(any(
    all(feature = "tokio", feature = "async-std"),
    not(any(feature = "tokio", feature = "async-std"))
))]
compile_error!("Either feature \"tokio\" or \"async-std\" must be enabled for this crate.");

#[cfg(any(feature = "pgp-commands", feature = "pgp-native"))]
#[cfg(any(
    all(feature = "rustls", feature = "openssl"),
    not(any(feature = "rustls", feature = "openssl"))
))]
compile_error!("Either feature \"rustls\" or \"openssl\" must be enabled for this crate.");
