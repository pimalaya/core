#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "client")]
pub mod client;
pub(crate) mod handler;
pub mod request;
pub mod response;
#[cfg(feature = "server")]
pub mod server;
#[cfg(any(feature = "tcp-binder", feature = "tcp-client"))]
pub mod tcp;
pub mod timer;
