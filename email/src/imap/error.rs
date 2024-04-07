use std::{any::Any, result};
use thiserror::Error;

use crate::{account, AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot execute operation on imap server")]
    ExecuteOperationError(#[source] imap::Error),
    #[error("cannot execute no-operation on imap server")]
    ExecuteNoOperationError(#[source] imap::Error),
    #[error("cannot authenticate to imap server")]
    AuthenticateImapError(#[source] imap::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdImapError(#[source] secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyImapError,
    #[error("cannot reset imap password")]
    ResetPasswordError(#[source] account::Error),
    #[error("cannot reset oauth secrets")]
    ResetOAuthSecretsError(#[source] account::Error),
    #[error("cannot refresh oauth access token")]
    RefreshAccessTokenError(#[source] account::Error),
    #[error("cannot login to imap server")]
    LoginImapError(#[source] imap::Error),
    #[error("cannot connect to imap server")]
    ConnectImapError(#[source] imap::Error),
    #[error("cannot get access token: {0}")]
    AccessTokenNotAvailable(#[source] account::Error),
    #[error("replacing unidentified to keyring failed: {0}")]
    ReplacingUnidentifiedFailed(#[source] secret::Error),

    #[error("cannot execute imap action after 3 retries")]
    ExecuteActionRetryError(#[source] AnyBoxedError),
    #[error("cannot execute imap action due to password authentication failure")]
    ExecuteActionPasswordError(#[source] AnyBoxedError),
    #[error("cannot execute imap action due to oauth authorization failure")]
    ExecuteActionOAuthError(#[source] AnyBoxedError),
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
