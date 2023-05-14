#![doc = include_str!("../README.md")]

pub mod evaluator;
pub mod lexer;
mod parser;
pub mod tpl;

pub use evaluator::{CompilerBuilder, CompilerOpts};
pub use tpl::{Tpl, TplBuilder};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    EvaluatorError(#[from] evaluator::Error),
    #[error(transparent)]
    LexerPartError(#[from] lexer::part::Error),
    #[error(transparent)]
    LexerTplError(#[from] lexer::tpl::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Compiles the given string template using default options. To
/// specify options, use [`CompilerBuilder`] instead.
pub fn compile<T: AsRef<str>>(tpl: T) -> Result<Vec<u8>> {
    CompilerBuilder::default().compile(tpl)
}
