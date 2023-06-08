mod config;
pub mod sendmail;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

use std::{borrow::Cow, result};
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

pub type Result<T> = result::Result<T, Error>;

pub trait Sender {
    fn send(&mut self, msg: &[u8]) -> Result<()>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SenderBuilder<'a> {
    account_config: Cow<'a, AccountConfig>,
}

impl<'a> SenderBuilder<'a> {
    pub fn new(account_config: Cow<'a, AccountConfig>) -> Self {
        Self { account_config }
    }

    pub fn build_into(self) -> Result<Box<dyn Sender + 'a>> {
        match self.account_config.sender.clone() {
            SenderConfig::None => Err(Error::BuildUndefinedSenderError),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => Ok(Box::new(Smtp::new(
                self.account_config,
                Cow::Owned(smtp_config),
            )?)),
            SenderConfig::Sendmail(sendmail_config) => Ok(Box::new(Sendmail::new(
                self.account_config,
                Cow::Owned(sendmail_config),
            ))),
        }
    }

    pub fn build(&'a self) -> Result<Box<dyn Sender + 'a>> {
        match &self.account_config.sender {
            SenderConfig::None => Err(Error::BuildUndefinedSenderError),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => Ok(Box::new(Smtp::new(
                Cow::Borrowed(&self.account_config),
                Cow::Borrowed(smtp_config),
            )?)),
            SenderConfig::Sendmail(sendmail_config) => Ok(Box::new(Sendmail::new(
                Cow::Borrowed(&self.account_config),
                Cow::Borrowed(sendmail_config),
            ))),
        }
    }
}
