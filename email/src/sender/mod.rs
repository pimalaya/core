mod config;
pub mod sendmail;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

pub use config::SenderConfig;
pub use sendmail::{Sendmail, SendmailConfig};
#[cfg(feature = "smtp-sender")]
pub use smtp::{Smtp, SmtpAuthConfig, SmtpConfig};

use std::result;
use thiserror::Error;

use crate::{account, email, AccountConfig};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build undefined sender")]
    BuildUndefinedSenderError,
    #[error(transparent)]
    EmailError(#[from] email::Error),
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[cfg(feature = "smtp-sender")]
    #[error(transparent)]
    SmtpError(#[from] smtp::Error),
    #[error(transparent)]
    SendmailError(#[from] sendmail::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub trait Sender {
    fn send(&mut self, msg: &[u8]) -> Result<()>;
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SenderBuilder;

impl SenderBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(self, account_config: &AccountConfig) -> Result<Box<dyn Sender>> {
        match &account_config.sender {
            SenderConfig::None => Err(Error::BuildUndefinedSenderError),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => {
                Ok(Box::new(Smtp::new(&account_config, &smtp_config)?))
            }
            SenderConfig::Sendmail(sendmail_config) => {
                Ok(Box::new(Sendmail::new(&account_config, &sendmail_config)))
            }
        }
    }
}
