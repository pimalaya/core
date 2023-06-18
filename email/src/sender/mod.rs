//! Module dedicated to senders.

mod config;
pub mod sendmail;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

use thiserror::Error;

use crate::{AccountConfig, Result};

#[cfg(feature = "smtp-sender")]
pub use self::smtp::{Smtp, SmtpAuthConfig, SmtpConfig};
pub use self::{
    config::SenderConfig,
    sendmail::{Sendmail, SendmailConfig},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build undefined sender")]
    BuildUndefinedSenderError,
}

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
            SenderConfig::None => Ok(Err(Error::BuildUndefinedSenderError)?),
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
            SenderConfig::None => Ok(Err(Error::BuildUndefinedSenderError)?),
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
