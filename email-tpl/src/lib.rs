#![doc = include_str!("../README.md")]

pub mod mml;
pub mod tpl;

pub use tpl::Interpreter as TplInterpreter;
pub use tpl::{Error, Result, Tpl};
