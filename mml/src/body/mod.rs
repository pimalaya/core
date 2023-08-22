#![allow(dead_code)]

#[cfg(feature = "compiler")]
pub mod compiler;
#[cfg(feature = "interpreter")]
pub mod interpreter;

#[cfg(feature = "compiler")]
pub use compiler::MmlBodyCompiler;
#[cfg(feature = "interpreter")]
pub use interpreter::{FilterParts, MmlBodyInterpreter};

pub(crate) const SINGLE_PART_BEGIN: &str = "<#part";
pub(crate) const SINGLE_PART_BEGIN_ESCAPED: &str = "<#!part";
pub(crate) const SINGLE_PART_END: &str = "<#/part>";
pub(crate) const SINGLE_PART_END_ESCAPED: &str = "<#!/part>";
pub(crate) const MULTI_PART_BEGIN: &str = "<#multipart";
pub(crate) const MULTI_PART_BEGIN_ESCAPED: &str = "<#!multipart";
pub(crate) const MULTI_PART_END: &str = "<#/multipart>";
pub(crate) const MULTI_PART_END_ESCAPED: &str = "<#!/multipart>";

pub(crate) const ALTERNATIVE: &str = "alternative";
pub(crate) const ATTACHMENT: &str = "attachment";
pub(crate) const DISPOSITION: &str = "disposition";
#[cfg(feature = "pgp")]
pub(crate) const ENCRYPT: &str = "encrypt";
pub(crate) const FILENAME: &str = "filename";
pub(crate) const INLINE: &str = "inline";
pub(crate) const MIXED: &str = "mixed";
pub(crate) const NAME: &str = "name";
#[cfg(feature = "pgp")]
pub(crate) const PGP_MIME: &str = "pgpmime";
pub(crate) const RELATED: &str = "related";
#[cfg(feature = "pgp")]
pub(crate) const SIGN: &str = "sign";
pub(crate) const TYPE: &str = "type";

pub(crate) const BACKSLASH: char = '\\';
pub(crate) const DOUBLE_QUOTE: char = '"';
pub(crate) const GREATER_THAN: char = '>';
pub(crate) const NEW_LINE: char = '\n';
pub(crate) const SPACE: char = ' ';
