#![doc = include_str!("../README.md")]

pub mod mml;
pub mod tpl;

pub use mml::interpreter::ShowPartsStrategy;
pub use tpl::{
    interpreter::ShowHeadersStrategy, Error, Interpreter as TplInterpreter, Result, Tpl,
};
