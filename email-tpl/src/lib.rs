#![doc = include_str!("../README.md")]

mod mml;
mod tpl;

pub use mml::compiler::CompilerBuilder;
pub use mml::interpreter::InterpreterBuilder;
pub use tpl::Tpl;
