//! Sendmail module.
//!
//! This module contains the representation of the sendmail email
//! sender.

use mailparse::MailParseError;
use std::result;
use thiserror::Error;

use crate::{process, sender, AccountConfig, Sender, SendmailConfig};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run sendmail command")]
    RunCmdError(#[source] process::Error),
    #[error("cannot parse email before sending")]
    ParseEmailError(#[source] MailParseError),
    #[error("cannot execute pre-send hook")]
    ExecutePreSendHookError(#[source] process::Error),
}

pub type Result<T> = result::Result<T, Error>;

pub struct Sendmail<'a> {
    account_config: &'a AccountConfig,
    sendmail_config: &'a SendmailConfig,
}

impl<'a> Sendmail<'a> {
    pub fn new(account_config: &'a AccountConfig, sendmail_config: &'a SendmailConfig) -> Self {
        Self {
            account_config,
            sendmail_config,
        }
    }
}

impl<'a> Sender for Sendmail<'a> {
    fn send(&mut self, email: &[u8]) -> sender::Result<()> {
        let mut email = mailparse::parse_mail(email).map_err(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.account_config.email_hooks.pre_send.as_deref() {
            buffer = process::run(cmd, email.raw_bytes).map_err(Error::ExecutePreSendHookError)?;
            email = mailparse::parse_mail(&buffer).map_err(Error::ParseEmailError)?;
        };

        process::run(&self.sendmail_config.cmd, email.raw_bytes).map_err(Error::RunCmdError)?;
        Ok(())
    }
}
