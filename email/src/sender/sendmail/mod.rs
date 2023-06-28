//! Module dedicated to the sendmail sender.
//!
//! This module contains the implementation of the sendmail sender and
//! all associated structures related to it.

pub mod config;

use mail_parser::Message;
use thiserror::Error;

use crate::{AccountConfig, Result, Sender};

pub use self::config::SendmailConfig;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run sendmail command")]
    RunSendmailCmdError(#[source] pimalaya_process::Error),
    #[error("cannot execute pre-send hook")]
    ExecutePreSendHookError(#[source] pimalaya_process::Error),
    #[error("cannot parse email before sending")]
    ParseEmailError,
}

/// The sendmail sender.
pub struct Sendmail {
    account_config: AccountConfig,
    sendmail_config: SendmailConfig,
}

impl Sendmail {
    /// Creates a new sendmail sender from configurations.
    pub fn new(account_config: AccountConfig, sendmail_config: SendmailConfig) -> Self {
        Self {
            account_config,
            sendmail_config,
        }
    }

    /// Sends the given raw email.
    pub fn send(&mut self, email: &[u8]) -> Result<()> {
        let mut email = Message::parse(&email).ok_or(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.account_config.email_hooks.pre_send.as_ref() {
            buffer = cmd
                .run_with(email.raw_message())
                .map_err(Error::ExecutePreSendHookError)?
                .stdout;
            email = Message::parse(&buffer).ok_or(Error::ParseEmailError)?;
        };

        self.sendmail_config
            .cmd
            .run_with(email.raw_message())
            .map_err(Error::RunSendmailCmdError)?;

        Ok(())
    }
}

impl Sender for Sendmail {
    fn send(&mut self, email: &[u8]) -> Result<()> {
        self.send(email)
    }
}
