#![cfg_attr(docs_rs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "client")]
pub mod client;
pub(crate) mod handler;
pub mod request;
pub mod response;
#[cfg(feature = "server")]
pub mod server;
#[cfg(feature = "tcp-any")]
pub mod tcp;
pub mod timer;
