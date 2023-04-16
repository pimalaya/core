#[cfg(feature = "smtp-sender")]
pub mod config;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

#[cfg(feature = "smtp-sender")]
pub use config::SmtpConfig;
#[cfg(feature = "smtp-sender")]
pub use smtp::{Error, Smtp};
