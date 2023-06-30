//! Module dedicated to sender management.
//!
//! The core concept of this module is the [`Sender`] trait, which is
//! an abstraction over emails sending.
//!
//! Then you have the [`SenderConfig`] which represents the
//! sender-specific configuration, mostly used by the [account
//! configuration](crate::AccountConfig).

mod config;
pub mod sendmail;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

use async_trait::async_trait;
use thiserror::Error;

use crate::{account::AccountConfig, Result};

#[doc(inline)]
pub use self::config::SenderConfig;
#[doc(inline)]
pub use self::sendmail::{Sendmail, SendmailConfig};
#[cfg(feature = "smtp-sender")]
#[doc(inline)]
pub use self::smtp::{Smtp, SmtpAuthConfig, SmtpConfig};

/// Errors related to senders.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build undefined sender")]
    BuildUndefinedSenderError,
}

/// The sender abstraction.
///
/// The sender trait abstracts the action of sending raw messages.
#[async_trait]
pub trait Sender {
    /// Sends the given raw message.
    ///
    /// The message needs to be a valid MIME Message. You can build
    /// one by compiling a [template](crate::email::template).
    async fn send(&mut self, msg: &[u8]) -> Result<()>;
}

/// The sender builder.
///
/// This builder helps you to build a `Box<dyn Sender>`. The type of
/// sender depends on the given account configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SenderBuilder {
    account_config: AccountConfig,
}

impl SenderBuilder {
    /// Creates a new builder with default value.
    pub fn new(account_config: AccountConfig) -> Self {
        Self { account_config }
    }

    /// Builds a [`Sender`] by cloning self options.
    pub async fn build(&self) -> Result<Box<dyn Sender>> {
        match &self.account_config.sender {
            SenderConfig::None => Ok(Err(Error::BuildUndefinedSenderError)?),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => Ok(Box::new(
                Smtp::new(self.account_config.clone(), smtp_config.clone()).await?,
            )),
            SenderConfig::Sendmail(sendmail_config) => Ok(Box::new(Sendmail::new(
                self.account_config.clone(),
                sendmail_config.clone(),
            ))),
        }
    }

    /// Builds a [`Sender`] by moving self options.
    pub async fn into_build(self) -> Result<Box<dyn Sender>> {
        match self.account_config.sender.clone() {
            SenderConfig::None => Ok(Err(Error::BuildUndefinedSenderError)?),
            #[cfg(feature = "smtp-sender")]
            SenderConfig::Smtp(smtp_config) => {
                Ok(Box::new(Smtp::new(self.account_config, smtp_config).await?))
            }
            SenderConfig::Sendmail(sendmail_config) => Ok(Box::new(Sendmail::new(
                self.account_config,
                sendmail_config,
            ))),
        }
    }
}
