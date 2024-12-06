use std::{any::Any, collections::HashSet, result};

use imap_client::{
    client::tokio::ClientError,
    imap_next::{
        client::Error as ClientFlowError,
        imap_types::{auth::AuthMechanism, error::ValidationError},
    },
    stream::Error as StreamError,
};
use thiserror::Error;
use tokio::task::JoinError;

use crate::{account, AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot build IMAP client: missing TLS provider")]
    BuildTlsClientMissingProvider,
    #[error("cannot build IMAP client")]
    JoinClientError(#[source] JoinError),
    #[error("cannot build IMAP client")]
    BuildClientError(#[source] Box<Error>),
    #[error("cannot connect to IMAP server {1}:{2} using TCP")]
    BuildInsecureClientError(#[source] ClientError, String, u16),
    #[error("cannot connect to IMAP server {1}:{2} using STARTTLS")]
    BuildStartTlsClientError(#[source] ClientError, String, u16),
    #[error("cannot connect to IMAP server {1}:{2} using SSL/TLS")]
    BuildTlsClientError(#[source] ClientError, String, u16),

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

    #[error("cannot send IMAP request")]
    RequestRetryError(#[source] ClientError),
    #[error("cannot send IMAP request")]
    ClientRetryError(#[source] ClientError),
    #[error("cannot send IMAP request: request timed out after 3 attempts")]
    RequestRetryTimeoutError,
    #[error("cannot enable IMAP capability")]
    EnableCapabilityError(#[source] ClientError),
    #[error("cannot authenticate to IMAP server: no valid auth mechanism found")]
    AuthenticateError(#[source] ClientError),
    #[error("cannot authenticate to IMAP server using LOGIN mechanism")]
    LoginError(#[source] ClientError),
    #[error("cannot authenticate to IMAP server using SASL PLAIN mechanism")]
    AuthenticatePlainError(#[source] ClientError),
    #[error("cannot authenticate to IMAP server using SASL XOAUTH2 mechanism")]
    AuthenticateXOauth2Error(#[source] ClientError),
    #[error("cannot authenticate to IMAP server using SASL OAUTHBEARER mechanism")]
    AuthenticateOAuthBearerError(#[source] ClientError),

    #[error("cannot create IMAP mailbox")]
    CreateMailboxError(#[source] ClientError),
    #[error("cannot create IMAP mailbox: request timed out")]
    CreateMailboxTimedOutError,

    #[error("cannot select IMAP mailbox")]
    SelectMailboxError(#[source] ClientError),
    #[error("cannot select IMAP mailbox: request timed out")]
    SelectMailboxTimedOutError,

    #[error("cannot examine IMAP mailbox")]
    ExamineMailboxError(#[source] ClientError),
    #[error("cannot examine IMAP mailbox: request timed out")]
    ExamineMailboxTimedOutError,

    #[error("cannot list IMAP mailboxes")]
    ListMailboxesError(#[source] ClientError),
    #[error("cannot list IMAP mailboxes: request timed out")]
    ListMailboxesTimedOutError,

    #[error("cannot expunge selected IMAP mailbox")]
    ExpungeMailboxError(#[source] ClientError),
    #[error("cannot expunge selected IMAP mailbox: request timed out")]
    ExpungeMailboxTimedOutError,

    #[error("cannot delete IMAP mailbox")]
    DeleteMailboxError(#[source] ClientError),
    #[error("cannot delete IMAP mailbox: request timed out")]
    DeleteMailboxTimedOutError,

    #[error("cannot fetch IMAP messages")]
    FetchMessagesError(#[source] ClientError),
    #[error("cannot fetch IMAP messages: request timed out")]
    FetchMessagesTimedOutError,

    #[error("cannot thread IMAP messages")]
    ThreadMessagesError(#[source] ClientError),
    #[error("cannot thread IMAP messages: request timed out")]
    ThreadMessagesTimedOutError,

    #[error("cannot store IMAP flag(s)")]
    StoreFlagsError(#[source] ClientError),
    #[error("cannot store IMAP flag(s): request timed out")]
    StoreFlagsTimedOutError,
    #[error("cannot add IMAP message")]
    AddMessageError(#[source] ClientError),
    #[error("cannot add IMAP message: request timed out")]
    AddMessageTimedOutError,
    #[error("cannot copy IMAP message(s)")]
    CopyMessagesError(#[source] ClientError),
    #[error("cannot copy IMAP message(s): request timed out")]
    CopyMessagesTimedOutError,
    #[error("cannot move IMAP message(s)")]
    MoveMessagesError(#[source] ClientError),
    #[error("cannot move IMAP message(s): request timed out")]
    MoveMessagesTimedOutError,
    #[error("cannot execute no-operation")]
    NoOpError(#[source] ClientError),
    #[error("cannot execute no-operation: request timed out")]
    NoOpTimedOutError,

    #[error("cannot exchange IMAP client/server ids")]
    ExchangeIdsError(#[source] ClientError),
    #[error("cannot search IMAP messages")]
    SearchMessagesError(#[source] ClientError),
    #[error("cannot sort IMAP messages")]
    SortMessagesError(#[source] ClientError),
    #[error("cannot sort IMAP envelope UIDs")]
    SortUidsError(#[source] ClientError),
    #[error("cannot sort IMAP envelope UIDs: request timed out")]
    SortUidsTimedOutError,
    #[error("cannot search IMAP envelope UIDs")]
    SearchUidsError(#[source] ClientError),
    #[error("cannot search IMAP envelope UIDs: request timed out")]
    SearchUidsTimedOutError,
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
    #[error("cannot execute IMAP no-op")]
    ExecuteNoOpError(#[source] ClientError),

    // flow
    #[error("cannot receive IMAP greeting")]
    ReceiveGreetingTaskError(#[source] ClientFlowError),
    #[error("login not supported")]
    LoginNotSupportedError,
    #[error("plain authentication not supported (available: {0:?})")]
    AuthenticatePlainNotSupportedError(HashSet<AuthMechanism<'static>>),
    #[error("XOAuth2 authentication not supported (available: {0:?})")]
    AuthenticateXOAuth2NotSupportedError(HashSet<AuthMechanism<'static>>),
    #[error("OAuthBearer authentication not supported (available: {0:?})")]
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
