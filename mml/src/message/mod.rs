//! # Message module
//!
//! A message is composed of a [header] and a [body]. A MML
//! message/body can be compiled into a MIME message/body using the
//! [MmlCompiler]/[MmlBodyCompiler] builder. A MIME message/body can
//! be interpreted as a MML message/body using the
//! [MimeInterpreter]/[MimeBodyInterpreter] builder.

pub mod body;
#[cfg(feature = "compiler")]
pub mod compiler;
pub(crate) mod header;
#[cfg(feature = "interpreter")]
pub mod interpreter;

#[cfg(feature = "compiler")]
#[doc(inline)]
pub use self::{body::MmlBodyCompiler, compiler::MmlCompiler};
#[cfg(feature = "interpreter")]
#[doc(inline)]
pub use self::{
    body::{FilterParts, MimeBodyInterpreter},
    interpreter::{MimeInterpreter, ShowHeadersStrategy},
};
