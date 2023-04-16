//! Sender module.
//!
//! This module contains the sender interface.

use std::result;
use thiserror::Error;

use crate::{account, email, sendmail, AccountConfig, EmailSender, Sendmail};

#[cfg(feature = "smtp-sender")]
use crate::{smtp, Smtp};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build email sender: sender is not defined")]
    BuildEmailSenderMissingError,

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
    fn send(&mut self, mime_msg: &[u8]) -> Result<()>;
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct SenderBuilder;

impl<'a> SenderBuilder {
    pub fn build(account_config: &'a AccountConfig) -> Result<Box<dyn Sender + 'a>> {
        match &account_config.email_sender {
            #[cfg(feature = "smtp-sender")]
            EmailSender::Smtp(smtp_config) => Ok(Box::new(Smtp::new(account_config, smtp_config))),
            EmailSender::Sendmail(sendmail_config) => {
                Ok(Box::new(Sendmail::new(account_config, sendmail_config)))
            }
            EmailSender::None => return Err(Error::BuildEmailSenderMissingError),
        }
    }
}
