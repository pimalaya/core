pub mod config;
pub mod sendmail;

pub use config::SendmailConfig;
pub use sendmail::{Error, Sendmail};
