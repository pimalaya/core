#[cfg(feature = "native-tls")]
pub extern crate native_tls as tls;
#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
pub extern crate rustls as tls;

pub mod backend;
pub mod domain;
pub mod sender;

pub use backend::*;
pub use domain::*;
pub use pimalaya_email_tpl::*;
pub use sender::*;
