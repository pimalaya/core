pub mod config;

use async_trait::async_trait;
use log::info;
use std::sync::Arc;

#[cfg(feature = "message-send")]
use crate::message::send::{sendmail::SendSendmailMessage, SendMessage};
use crate::{
    account::config::AccountConfig,
    backend::{BackendContext, BackendContextBuilder, BackendFeatureBuilder},
    Result,
};

use self::config::SendmailConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SendmailContext {
    pub account_config: Arc<AccountConfig>,
    pub sendmail_config: Arc<SendmailConfig>,
}

impl SendmailContext {
    pub fn new(account_config: Arc<AccountConfig>, sendmail_config: Arc<SendmailConfig>) -> Self {
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
    /// The account configuration.
    pub account_config: Arc<AccountConfig>,

    /// The sendmail configuration.
    pub sendmail_config: Arc<SendmailConfig>,
}

impl SendmailContextBuilder {
    pub fn new(account_config: Arc<AccountConfig>, sendmail_config: Arc<SendmailConfig>) -> Self {
        Self {
            account_config,
            sendmail_config,
        }
    }
}

#[async_trait]
impl BackendContextBuilder for SendmailContextBuilder {
    type Context = SendmailContextSync;

    #[cfg(feature = "message-send")]
    fn send_message(&self) -> BackendFeatureBuilder<Self::Context, dyn SendMessage> {
        Some(Arc::new(SendSendmailMessage::some_new_boxed))
    }

    /// Build an SENDMAIL sync session.
    ///
    /// The SENDMAIL session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self) -> Result<Self::Context> {
        info!("building new sendmail context");

        Ok(SendmailContextSync {
            account_config: self.account_config,
            sendmail_config: self.sendmail_config,
        })
    }
}
