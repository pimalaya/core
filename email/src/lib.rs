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

#[cfg(all(feature = "native-tls", not(feature = "rustls-tls")))]
pub extern crate native_tls as tls;

#[cfg(feature = "rustls-tls")]
pub extern crate rustls as tls;

#[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
compile_error!("Exactly one of 'native-tls' or 'rustls-tls' feature has to be enabled!");