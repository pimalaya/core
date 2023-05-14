#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
pub extern crate rustls as tls;

#[cfg(feature = "native-tls")]
pub extern crate native_tls as tls;

pub mod backend;
pub mod domain;
pub(crate) mod process;
pub mod sender;

pub use backend::*;
pub use domain::*;
pub use pimalaya_email_tpl::{
    evaluator::CompilerBuilder,
    tpl::{HeaderVal, ShowHeaders, ShowTextPartsStrategy, Tpl, TplBuilder},
};
pub use sender::*;
