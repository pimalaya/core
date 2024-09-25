use std::{any::Any, result};

use thiserror::Error;

use crate::{AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot send message without a sender")]
    SendMessageMissingSenderError,
    #[error("cannot send message without a recipient")]
    SendMessageMissingRecipientError,
    #[error("cannot send message: request timed out")]
    SendMessageTimedOutError,
    #[error("cannot send message")]
    SendMessageError(#[source] mail_send::Error),
    #[error("cannot connect to smtp server using tcp")]
    ConnectTcpSmtpError(#[source] mail_send::Error),
    #[error("cannot connect to smtp server using tls")]
    ConnectTlsSmtpError(#[source] mail_send::Error),
    #[error("cannot get smtp password")]
    GetPasswdSmtpError(#[source] secret::Error),
    #[error("cannot get smtp password: password is empty")]
    GetPasswdEmptySmtpError,
    #[error("cannot get access token")]
    AccessTokenWasNotAvailable,
    #[error("cannot refresh access token")]
    RefreshingAccessTokenFailed,
    #[error("resetting oauth failed")]
    ResettingOAuthFailed,
    #[error("configuring oauth failed")]
    ConfiguringOAuthFailed,
    #[error("replacing keyring failed: {0}")]
    ReplacingKeyringFailed(#[source] secret::Error),
    #[error("mail send noop failed: {0}")]
    MailSendNoOpFailed(#[source] mail_send::Error),
}

impl AnyError for Error {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<Error> for AnyBoxedError {
    fn from(err: Error) -> Self {
        Box::new(err)
    }
}
