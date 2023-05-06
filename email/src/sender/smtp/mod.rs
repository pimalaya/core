pub mod config;
pub mod smtp;

pub use config::{SmtpAuthConfig, SmtpConfig};
pub use smtp::{Error, Smtp};
