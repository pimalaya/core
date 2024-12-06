use std::{any::Any, io, path::PathBuf, result};

use chumsky::error::Rich;
#[cfg(feature = "imap")]
use imap_client::imap_next::imap_types::error::ValidationError;
use thiserror::Error;
use tokio::task::JoinError;

#[cfg(feature = "maildir")]
use crate::flag::Flags;
use crate::{
    envelope::{Id, SingleId},
    AnyBoxedError, AnyError,
};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[cfg(feature = "imap")]
    #[error("cannot sort IMAP ids using {1:?} and {2:?}")]
    SortUidsError(
        #[source] imap_client::client::tokio::ClientError,
        imap_client::imap_next::imap_types::core::Vec1<
            imap_client::imap_next::imap_types::search::SearchKey<'static>,
        >,
        imap_client::imap_next::imap_types::core::Vec1<
            imap_client::imap_next::imap_types::extensions::sort::SortCriterion,
        >,
    ),
    #[cfg(feature = "imap")]
    #[error("cannot search IMAP ids using {1:?}")]
    SearchUidsError(
        #[source] imap_client::client::tokio::ClientError,
        imap_client::imap_next::imap_types::core::Vec1<
            imap_client::imap_next::imap_types::search::SearchKey<'static>,
        >,
    ),

    #[cfg(feature = "imap")]
    #[error("cannot parse IMAP sequence")]
    ParseSequenceError(#[source] ValidationError),
    #[cfg(feature = "maildir")]
    #[error("cannot list maildir entries")]
    ListMaildirEntriesError(#[source] maildirs::Error),
    #[cfg(feature = "maildir")]
    #[error("cannot get flags from maildir entry {0}")]
    GetMaildirFlagsError(#[source] maildirs::Error, PathBuf),
    #[error("cannot find message associated to envelope {0}")]
    FindMessageError(String),
    #[error("cannot parse search emails query `{1}`")]
    ParseError(Vec<Rich<'static, char>>, String),
    #[error("cannot interpret message as template")]
    InterpretMessageAsTemplateError(#[source] mml::Error),
    #[error("cannot interpret message as thread template")]
    InterpretMessageAsThreadTemplateError(#[source] mml::Error),
    #[error("cannot run sendmail command")]
    RunSendmailCommandError(#[source] process::Error),
    #[cfg(feature = "notmuch")]
    #[error("cannot remove notmuch message(s) {2} from folder {1}")]
    RemoveNotmuchMessageError(#[source] notmuch::Error, String, Id),
    #[cfg(feature = "maildir")]
    #[error("cannot remove maildir message(s) {2} from folder {1}")]
    RemoveMaildirMessageError(#[source] maildirs::Error, String, String),
    #[cfg(feature = "notmuch")]
    #[error("cannot move notmuch message {3} from {1} to {2}")]
    MoveMessageNotmuchError(#[source] notmuch::Error, String, String, String),
    #[cfg(feature = "maildir")]
    #[error("cannot move message {3} from maildir folder {1} to folder {2}")]
    MoveMessagesMaildirError(#[source] maildirs::Error, String, String, PathBuf),
    #[error("cannot parse email")]
    ParseEmailError,
    #[error("cannot parse email: raw email is empty")]
    ParseEmailEmptyRawError,
    #[error("cannot delete local draft at {1}")]
    DeleteLocalDraftError(#[source] io::Error, PathBuf),
    #[error("cannot parse email: empty entries")]
    ParseEmailFromEmptyEntriesError,
    #[error("could not parse: {0}")]
    ChumskyError(String),
    #[error(transparent)]
    AcountError(#[from] crate::account::Error),
    #[error("cannot decrypt encrypted email part")]
    DecryptEmailPartError(#[source] process::Error),
    #[error("cannot verify signed email part")]
    VerifyEmailPartError(#[source] process::Error),
    #[error("cannot get content type of multipart")]
    GetMultipartContentTypeError,
    #[error("cannot find encrypted part of multipart")]
    GetEncryptedPartMultipartError,
    #[error("cannot parse encrypted part of multipart")]
    WriteEncryptedPartBodyError(#[source] io::Error),
    #[error("cannot write encrypted part to temporary file")]
    DecryptPartError(#[source] crate::account::Error),
    #[error("cannot interpret email as template")]
    InterpretEmailAsTplError(#[source] mml::Error),
    #[error("cannot parse email message")]
    ParseEmailMessageError,
    #[error("cannot get notmuch message filename from {0}")]
    GetMessageFilenameNotmuchError(PathBuf),
    #[cfg(feature = "notmuch")]
    #[error("cannot copy notmuch message {3} from {1} to {2}")]
    CopyMessageNotmuchError(#[source] notmuch::Error, String, String, String),
    #[cfg(feature = "maildir")]
    #[error("cannot copy maildir message {3} from folder {1} to folder {2}")]
    CopyMessagesMaildirError(#[source] maildirs::Error, String, String, PathBuf),
    #[cfg(feature = "maildir")]
    #[error("cannot add maildir message to folder {1} with flags {2}")]
    StoreWithFlagsMaildirError(#[source] maildirs::Error, String, Flags),
    #[error("cannot get added imap message uid from range {0}")]
    GetAddedMessageUidFromRangeImapError(String),
    #[error("cannot get added imap message uid: extension UIDPLUS may be missing on the server")]
    GetAddedMessageUidImapError,
    #[cfg(feature = "maildir")]
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderMaildirError(#[source] maildirs::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderMaildirError(PathBuf, PathBuf),
    #[cfg(feature = "maildir")]
    #[error("cannot create maildir {1} folder structure")]
    InitFolderMaildirError(#[source] maildirs::Error, PathBuf),
    #[error("cannot list notmuch envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsNotmuchError(String, usize),
    #[cfg(feature = "notmuch")]
    #[error("cannot list notmuch envelopes from {0}: invalid query {1}")]
    SearchMessagesInvalidQueryNotmuch(#[source] notmuch::Error, String, String),
    #[error("cannot list maildir envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsMaildirError(String, usize),
    #[error("cannot list imap envelopes: page {0} out of bounds")]
    BuildPageRangeOutOfBoundsImapError(usize),
    #[error("cannot get uid of imap envelope {0}: uid is missing")]
    GetUidMissingImapError(u32),
    #[error("cannot get missing envelope {0}")]
    GetEnvelopeMissingError(u32),
    #[error("cannot find notmuch envelope {1} from folder {0}")]
    FindEnvelopeEmptyNotmuchError(String, String),
    #[error("cannot find maildir envelope {1:?} from folder {0}")]
    GetEnvelopeMaildirError(PathBuf, SingleId),
    #[error("cannot find imap envelope {1} from folder {0}")]
    GetFirstEnvelopeImapError(String, Id),
    #[cfg(feature = "maildir")]
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagsMaildirError(#[source] maildirs::Error, String, String, Flags),
    #[cfg(feature = "maildir")]
    #[error("cannot remove flags {3} to envelope(s) {2} from folder {1}")]
    RemoveFlagsMaildirError(#[source] maildirs::Error, String, String, Flags),
    #[error("cannot parse flag {0}")]
    ParseFlagError(String),
    #[error("cannot parse maildir flag {0}")]
    ParseFlagMaildirError(String),
    #[error("cannot parse imap flag {0}")]
    ParseFlagImapError(String),
    #[cfg(feature = "maildir")]
    #[error("cannot add maildir flags {3} to envelope(s) {2} from folder {1}")]
    AddFlagsMaildirError(#[source] maildirs::Error, String, String, Flags),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("failed to get envelopes: {0}")]
    FailedToGetEnvelopes(#[source] JoinError),
    #[cfg(feature = "notmuch")]
    #[error("notmuch failed: {0}")]
    NotMuchFailure(notmuch::Error),
    #[error("process failed: {0}")]
    ProcessFailure(process::Error),
    #[cfg(feature = "maildir")]
    #[error("maildir failed: {0}")]
    MaildirppFailure(maildirs::Error),
    #[cfg(feature = "maildir")]
    #[error("could not watch: {0}")]
    NotifyFailure(notify::Error),
    #[error("could not watch: {0}")]
    FileReadFailure(io::Error),

    #[error("cannot list envelopes from left sync cache")]
    ListLeftEnvelopesCachedError(#[source] AnyBoxedError),
    #[error("cannot list envelopes from left sync backend")]
    ListLeftEnvelopesError(#[source] AnyBoxedError),
    #[error("cannot list envelopes from right sync cache")]
    ListRightEnvelopesCachedError(#[source] AnyBoxedError),
    #[error("cannot list envelopes from right sync backend")]
    ListRightEnvelopesError(#[source] AnyBoxedError),

    #[cfg(feature = "maildir")]
    #[error(transparent)]
    MaildirsError(#[from] maildirs::Error),

    #[error(transparent)]
    IoError(#[from] io::Error),
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
