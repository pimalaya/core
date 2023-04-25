pub use mime_msg_builder::{
    evaluator::CompilerBuilder,
    tpl::{HeaderVal, ShowHeaders, ShowTextPartsStrategy, Tpl, TplBuilder},
};

pub(crate) mod process;

pub mod backend;
pub use backend::*;

pub mod sender;
pub use sender::*;

pub mod domain;
pub use domain::*;

#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
pub extern crate rustls as tls;

#[cfg(feature = "native-tls")]
pub extern crate native_tls as tls;
