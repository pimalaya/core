use thiserror::Error;
#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot authenticate to imap server")]
    AuthenticateImapError(#[source] imap::Error),
    #[error("cannot get imap password from global keyring")]
    GetPasswdImapError(#[source] secret::Error),
    #[error("cannot get imap password: password is empty")]
    GetPasswdEmptyImapError,
    #[error("cannot login to imap server")]
    LoginImapError(#[source] imap::Error),
    #[error("cannot connect to imap server")]
    ConnectImapError(#[source] imap::Error),
    #[error("cannot get access token: {0}")]
    AccessTokenNotAvailable(#[source] crate::account::error::Error),
    #[error("replacing unidentified to keyring failed: {0}")]
    ReplacingUnidentifiedFailed(#[source] secret::Error),
}
