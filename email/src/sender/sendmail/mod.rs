pub mod config;
pub use config::SendmailConfig;

use mailparse::MailParseError;
use pimalaya_process::Cmd;
use std::result;
use thiserror::Error;

use crate::{sender, AccountConfig, EmailHooks, Sender};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run sendmail command")]
    RunSendmailCmdError(#[source] pimalaya_process::Error),
    #[error("cannot execute pre-send hook")]
    ExecutePreSendHookError(#[source] pimalaya_process::Error),
    #[error("cannot parse email before sending")]
    ParseEmailError(#[source] MailParseError),
}

pub type Result<T> = result::Result<T, Error>;

pub struct Sendmail {
    hooks: EmailHooks,
    cmd: Cmd,
}

impl Sendmail {
    pub fn new(account_config: &AccountConfig, sendmail_config: &SendmailConfig) -> Self {
        Self {
            hooks: account_config.email_hooks.clone(),
            cmd: sendmail_config.cmd.clone(),
        }
    }
}

impl Sender for Sendmail {
    fn send(&self, email: &[u8]) -> sender::Result<()> {
        let mut email = mailparse::parse_mail(email).map_err(Error::ParseEmailError)?;
        let buffer;

        if let Some(cmd) = self.hooks.pre_send.as_ref() {
            buffer = cmd
                .run_with(email.raw_bytes)
                .map_err(Error::ExecutePreSendHookError)?
                .stdout;
            email = mailparse::parse_mail(&buffer).map_err(Error::ParseEmailError)?;
        };

        self.cmd
            .run_with(email.raw_bytes)
            .map_err(Error::RunSendmailCmdError)?;

        Ok(())
    }
}
