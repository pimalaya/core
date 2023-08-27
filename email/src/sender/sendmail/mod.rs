//! Module dedicated to the sendmail sender.
//!
//! This module contains the implementation of the sendmail sender and
//! all associated structures related to it.

pub mod config;

use async_trait::async_trait;
use log::{debug, warn};
use mail_parser::Message;
use thiserror::Error;

use crate::{account::AccountConfig, sender::Sender, Result};

#[doc(inline)]
pub use self::config::SendmailConfig;

/// Errors related to the sendmail sender.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot run sendmail command")]
    RunCommandError(#[source] process::Error),
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

    /// Sends the given raw message.
    pub async fn send(&mut self, msg: &[u8]) -> Result<()> {
        let buffer: Vec<u8>;
        let mut msg = Message::parse(&msg).unwrap_or_else(|| {
            warn!("cannot parse raw message");
            Default::default()
        });

        if let Some(cmd) = self.account_config.email_hooks.pre_send.as_ref() {
            match cmd.run_with(msg.raw_message()).await {
                Ok(res) => {
                    buffer = res.into();
                    msg = Message::parse(&buffer).unwrap_or_else(|| {
                        warn!("cannot parse raw message after pre-send hook");
                        Default::default()
                    });
                }
                Err(err) => {
                    warn!("cannot execute pre-send hook: {err}");
                    debug!("cannot execute pre-send hook {cmd:?}: {err:?}");
                }
            }
        };

        self.sendmail_config
            .cmd
            .run_with(msg.raw_message())
            .await
            .map_err(Error::RunCommandError)?;

        Ok(())
    }
}

#[async_trait]
impl Sender for Sendmail {
    async fn send(&mut self, msg: &[u8]) -> Result<()> {
        self.send(msg).await
    }
}
