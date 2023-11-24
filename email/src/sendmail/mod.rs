mod config;

use async_trait::async_trait;
use log::info;

use crate::{account::AccountConfig, backend::BackendContextBuilder, Result};

#[doc(inline)]
pub use self::config::SendmailConfig;

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

#[async_trait]
impl BackendContextBuilder for SendmailContext {
    type Context = Self;

    /// Build an SENDMAIL sync session.
    ///
    /// The SENDMAIL session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self) -> Result<Self::Context> {
        info!("building new sendmail context");
        Ok(self)
    }
}
