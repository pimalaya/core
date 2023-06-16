//! Module dedicated to senders.

mod config;
pub mod sendmail;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

use std::result;
use thiserror::Error;

pub use self::config::SenderConfig;
pub use self::sendmail::{Sendmail, SendmailConfig};
#[cfg(feature = "smtp-sender")]
pub use self::smtp::{Smtp, SmtpAuthConfig, SmtpConfig};
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

type Result<T> = result::Result<T, Error>;

pub trait Sender {
    fn send(&mut self, msg: &[u8]) -> Result<()>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SenderBuilder {
    account_config: AccountConfig,
}

impl SenderBuilder {
    pub fn new(account_config: AccountConfig) -> Self {
        Self { account_config }
    }

    pub fn build_into(self) -> Result<Box<dyn Sender>> {
        match self.account_config.sender.clone() {
            SenderConfig::None => Err(Error::BuildUndefinedSenderError),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => {
                Ok(Box::new(Smtp::new(self.account_config, smtp_config)?))
            }
            SenderConfig::Sendmail(sendmail_config) => Ok(Box::new(Sendmail::new(
                self.account_config,
                sendmail_config,
            ))),
        }
    }

    pub fn build(&self) -> Result<Box<dyn Sender>> {
        match &self.account_config.sender {
            SenderConfig::None => Err(Error::BuildUndefinedSenderError),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => Ok(Box::new(Smtp::new(
                self.account_config.clone(),
                smtp_config.clone(),
            )?)),
            SenderConfig::Sendmail(sendmail_config) => Ok(Box::new(Sendmail::new(
                self.account_config.clone(),
                sendmail_config.clone(),
            ))),
        }
    }
}
