#![doc = include_str!("../README.md")]

mod mml;
mod tpl;

pub use mml::compiler::*;
pub use mml::interpreter::*;
pub use tpl::Tpl;
