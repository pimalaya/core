pub mod config;
mod error;

use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use self::config::SendmailConfig;
#[doc(inline)]
pub use self::error::{Error, Result};
use crate::{
    account::config::AccountConfig,
    backend::{
        context::{BackendContext, BackendContextBuilder},
        feature::{BackendFeature, CheckUp},
    },
    message::send::{sendmail::SendSendmailMessage, SendMessage},
    AnyResult,
};

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

    fn check_up(&self) -> Option<BackendFeature<Self::Context, dyn CheckUp>> {
        Some(Arc::new(CheckUpSendmail::some_new_boxed))
    }

    fn send_message(&self) -> Option<BackendFeature<Self::Context, dyn SendMessage>> {
        Some(Arc::new(SendSendmailMessage::some_new_boxed))
    }

    /// Build an SENDMAIL sync session.
    ///
    /// The SENDMAIL session is created at this moment. If the session
    /// cannot be created using the OAuth 2.0 authentication, the
    /// access token is refreshed first then a new session is created.
    async fn build(self) -> AnyResult<Self::Context> {
        info!("building new sendmail context");

        Ok(SendmailContextSync {
            account_config: self.account_config,
            sendmail_config: self.sendmail_config,
        })
    }
}

#[derive(Clone)]
pub struct CheckUpSendmail {
    pub ctx: SendmailContextSync,
}

impl CheckUpSendmail {
    pub fn new(ctx: &SendmailContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &SendmailContext) -> Box<dyn CheckUp> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &SendmailContext) -> Option<Box<dyn CheckUp>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl CheckUp for CheckUpSendmail {
    async fn check_up(&self) -> AnyResult<()> {
        self.ctx
            .sendmail_config
            .cmd()
            .run()
            .await
            .map_err(Error::ExecuteCommandError)?;
        Ok(())
    }
}
