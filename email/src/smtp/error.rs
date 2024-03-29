use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot send message without a sender")]
    SendMessageMissingSenderError,
    #[error("cannot send message without a recipient")]
    SendMessageMissingRecipientError,
    #[error("cannot send message")]
    SendMessageSmtpError(#[source] mail_send::Error),
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
