pub mod config;

use async_trait::async_trait;
use log::info;

use crate::{
    account::config::AccountConfig,
    backend::{BackendContext, BackendContextBuilder},
    Result,
};

use self::config::SendmailConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SendmailContext {
    pub account_config: AccountConfig,
    pub sendmail_config: SendmailConfig,
}

impl SendmailContext {
    pub fn new(account_config: AccountConfig, sendmail_config: SendmailConfig) -> Self {
        Self {
            account_config,
            sendmail_config,
        }
    }
}

pub type SendmailContextSync = SendmailContext;

impl BackendContext for SendmailContextSync {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SendmailContextBuilder {
    /// The sendmail configuration
    pub config: SendmailConfig,
}

impl SendmailContextBuilder {
    pub fn new(config: SendmailConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl BackendContextBuilder for SendmailContextBuilder {
    type Context = SendmailContextSync;

    /// Build an SENDMAIL sync session.
    ///
    /// The SENDMAIL session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self, account_config: &AccountConfig) -> Result<Self::Context> {
        info!("building new sendmail context");

        Ok(SendmailContextSync {
            account_config: account_config.clone(),
            sendmail_config: self.config,
        })
    }
}
