//! # Message module
//!
//! A message is composed of a header and a [body].
//!
//! ## Compilation
//!
//! A MML message/body can be compiled into a MIME message/body using
//! the [MmlCompilerBuilder]/[MmlBodyCompiler] builders.
//!
//! ## Interpretation
//!
//! A MIME message/body can be interpreted as a MML message/body using
//! the [MimeInterpreterBuilder]/[MimeBodyInterpreter] builder.

pub mod body;
#[cfg(feature = "compiler")]
pub mod compiler;
pub(crate) mod header;
#[cfg(feature = "interpreter")]
pub mod interpreter;

#[cfg(feature = "compiler")]
#[doc(inline)]
pub use self::{
    body::MmlBodyCompiler,
    compiler::{MmlCompileResult, MmlCompiler, MmlCompilerBuilder},
};
#[cfg(feature = "interpreter")]
#[doc(inline)]
pub use self::{
    body::{FilterParts, MimeBodyInterpreter},
    interpreter::{FilterHeaders, MimeInterpreter, MimeInterpreterBuilder},
};
