#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

pub mod decrypt;
pub mod encrypt;
mod error;
#[cfg(feature = "key-discovery")]
pub mod http;
pub mod sign;
pub mod utils;
pub mod verify;

pub use pgp_native as native;

#[doc(inline)]
pub use crate::{
    decrypt::decrypt,
    encrypt::encrypt,
    error::{Error, Result},
    sign::sign,
    utils::{
        gen_key_pair, read_pkey_from_path, read_pkey_from_string, read_sig_from_bytes,
        read_skey_from_file, read_skey_from_string,
    },
    verify::verify,
};

#[cfg(feature = "key-discovery")]
#[cfg(any(
    all(feature = "tokio", feature = "async-std"),
    not(any(feature = "tokio", feature = "async-std"))
))]
compile_error!("Either feature `tokio` or `async-std` must be enabled for this crate.");

#[cfg(feature = "key-discovery")]
#[cfg(any(
    all(feature = "rustls", feature = "native-tls"),
    not(any(feature = "rustls", feature = "native-tls"))
))]
compile_error!("Either feature `rustls` or `native-tls` must be enabled for this crate.");
