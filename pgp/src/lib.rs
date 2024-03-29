#![doc = include_str!("../README.md")]

pub mod decrypt;
pub mod encrypt;
pub mod error;
pub mod hkp;
pub mod http;
pub mod sign;
pub mod utils;
pub mod verify;
pub mod wkd;

pub(crate) mod client;

pub use error::*;
#[doc(inline)]
pub use pgp_native as native;

#[doc(inline)]
pub use self::{
    decrypt::decrypt,
    encrypt::encrypt,
    sign::sign,
    utils::{
        gen_key_pair, read_pkey_from_path, read_sig_from_bytes, read_skey_from_file,
        read_skey_from_string,
    },
    verify::verify,
};
