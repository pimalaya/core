#![cfg_attr(docs_rs, feature(doc_cfg, doc_auto_cfg))]
#![doc = include_str!("../README.md")]

#[cfg(feature = "key-discovery")]
pub(crate) mod client;
pub mod decrypt;
pub mod encrypt;
mod error;
#[cfg(feature = "key-discovery")]
pub mod hkp;
#[cfg(feature = "key-discovery")]
pub mod http;
pub mod sign;
pub mod utils;
pub mod verify;
#[cfg(feature = "key-discovery")]
pub mod wkd;

pub use pgp_native as native;

#[doc(inline)]
pub use crate::{
    decrypt::decrypt,
    encrypt::encrypt,
    error::{Error, Result},
    sign::sign,
    utils::{
        gen_key_pair, read_pkey_from_path, read_sig_from_bytes, read_skey_from_file,
        read_skey_from_string,
    },
    verify::verify,
};
