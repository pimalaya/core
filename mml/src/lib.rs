//! # 📫 MIME Meta Language
//!
//! Rust implementation of the Emacs MIME message Meta Language, as
//! known as [MML].
//!
//! This library exposes a [MML to MIME](MmlCompilerBuilder) message
//! compiler and a [MIME to MML](MimeInterpreterBuilder) message
//! interpreter.
//!
//! For example:
//!
//! ```mml,ignore
#![doc = include_str!("../examples/main.mml.eml")]
//! ```
//!
//! compiles to:
//!
//! ```eml,ignore
#![doc = include_str!("../examples/main.mime.eml")]
//! ```
//!
//! See [more examples].
//!
//! [MML]: https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html
//! [more examples]: https://git.sr.ht/~soywod/pimalaya/tree/master/item/mml/examples

#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub mod error;
pub use error::*;
pub mod message;
#[cfg(feature = "pgp")]
pub mod pgp;

#[cfg(feature = "interpreter")]
#[doc(inline)]
pub use self::message::{MimeInterpreter, MimeInterpreterBuilder};
#[cfg(feature = "compiler")]
#[doc(inline)]
pub use self::message::{MmlCompileResult, MmlCompiler, MmlCompilerBuilder};
