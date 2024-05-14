use std::{any::Any, collections::HashSet, result};

use imap_client::{
    imap_flow::{
        client::ClientFlowError,
        imap_codec::imap_types::{auth::AuthMechanism, error::ValidationError},
        stream::StreamError,
    },
    ClientError,
};
use thiserror::Error;

use crate::{account, AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
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

    // ================ v2

    // parse
    #[error("cannot parse IMAP mailbox {1}")]
    ParseMailboxError(#[source] ValidationError, String),
    #[error("cannot find UID of appended IMAP message")]
    FindAppendedMessageUidError,

    #[error("cannot authenticate to IMAP server using PLAIN mechanism")]
    AuthenticatePlainError(#[source] ClientError),
    #[error("cannot authenticate to IMAP server using XOAUTH2 mechanism")]
    AuthenticateXOauth2Error(#[source] ClientError),
    #[error("cannot authenticate to IMAP server using OAUTHBEARER mechanism")]
    AuthenticateOAuthBearerError(#[source] ClientError),

    #[error("cannot create IMAP mailbox")]
    CreateMailboxError(#[source] ClientError),
    #[error("cannot select IMAP mailbox")]
    SelectMailboxError(#[source] ClientError),
    #[error("cannot examine IMAP mailbox")]
    ExamineMailboxError(#[source] ClientError),
    #[error("cannot list IMAP mailboxes")]
    ListMailboxesError(#[source] ClientError),
    #[error("cannot expunge selected IMAP mailbox")]
    ExpungeMailboxError(#[source] ClientError),
    #[error("cannot delete IMAP mailbox")]
    DeleteMailboxError(#[source] ClientError),

    #[error("cannot fetch IMAP messages")]
    FetchMessagesError(#[source] ClientError),
    #[error("cannot search IMAP messages")]
    SearchMessagesError(#[source] ClientError),
    #[error("cannot sort IMAP messages")]
    SortMessagesError(#[source] ClientError),
    #[error("cannot thread IMAP messages")]
    ThreadMessagesError(#[source] ClientError),
    #[error("cannot start IMAP IDLE mode")]
    StartIdleError(#[source] StreamError<ClientFlowError>),
    #[error("cannot stop IMAP IDLE mode")]
    StopIdleError(#[source] StreamError<ClientFlowError>),
    #[error("IMAP IDLE mode interrupted")]
    IdleInterruptedError,
    #[error("cannot append IMAP message")]
    AppendMessageError(#[source] ClientError),
    #[error("cannot execute IMAP no-op after append")]
    ExecuteNoOpAfterAppendError(#[source] ClientError),
    #[error("cannot execute IMAP check after append")]
    ExecuteCheckAfterAppendError(#[source] ClientError),
    #[error("cannot copy IMAP message(s)")]
    CopyMessagesError(#[source] ClientError),
    #[error("cannot move IMAP message(s)")]
    MoveMessagesError(#[source] ClientError),
    #[error("cannot store IMAP flag(s)")]
    StoreFlagsError(#[source] ClientError),
    #[error("cannot execute IMAP no-op")]
    ExecuteNoOpError(#[source] ClientError),

    // flow
    #[error("cannot receive IMAP greeting")]
    ReceiveGreetingTaskError(#[source] ClientFlowError),
    #[error("plain authentication not support (available: {0:?})")]
    AuthenticatePlainNotSupportedError(HashSet<AuthMechanism<'static>>),
    #[error("XOAuth2 authentication not support (available: {0:?})")]
    AuthenticateXOAuth2NotSupportedError(HashSet<AuthMechanism<'static>>),
    #[error("OAuthBearer authentication not support (available: {0:?})")]
    AuthenticateOAuthBearerNotSupportedError(HashSet<AuthMechanism<'static>>),

    // tasks
    #[error("cannot execute IMAP action")]
    ExecuteActionV2Error(#[source] AnyBoxedError),

    #[error("cannot build IMAP session after {0} attempts, aborting")]
    BuildSessionRetryError(u8),
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
