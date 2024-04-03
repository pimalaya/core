use thiserror::Error;

use crate::account;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot authenticate to imap server")]
    AuthenticateImapError(#[source] imap::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdImapError(#[source] secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyImapError,
    #[error("cannot reset imap password")]
    ResetPasswordError(#[source] account::error::Error),
    #[error("cannot reset oauth secrets")]
    ResetOAuthSecretsError(#[source] account::error::Error),
    #[error("cannot login to imap server")]
    LoginImapError(#[source] imap::Error),
    #[error("cannot connect to imap server")]
    ConnectImapError(#[source] imap::Error),
    #[error("cannot get access token: {0}")]
    AccessTokenNotAvailable(#[source] crate::account::error::Error),
    #[error("replacing unidentified to keyring failed: {0}")]
    ReplacingUnidentifiedFailed(#[source] secret::Error),
    #[error("this should not happen: {0}")]
    NoopFailure(#[source] imap::Error),
}

impl crate::EmailError for Error {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl From<Error> for Box<dyn crate::EmailError> {
    fn from(value: Error) -> Self {
        Box::new(value)
    }
}
