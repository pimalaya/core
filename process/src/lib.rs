#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

mod command;
mod error;
mod output;
mod pipeline;

#[doc(inline)]
pub use crate::{
    command::Command,
    error::{Error, Result},
    output::Output,
    pipeline::Pipeline,
};

#[cfg(any(
    all(feature = "tokio", feature = "async-std"),
    not(any(feature = "tokio", feature = "async-std"))
))]
compile_error!("Either feature `tokio` or `async-std` must be enabled for this crate.");
