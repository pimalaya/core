use chumsky::error::Rich;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use tokio::task::JoinError;

use crate::envelope::Id;
use crate::flag::Flags;

#[derive(Error, Debug)]
pub enum Error {
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
    #[error("cannot remove maildir message(s) {2} from folder {1}")]
    RemoveMaildirMessageError(#[source] maildirpp::Error, String, String),
    #[error("cannot add deleted flag to imap message(s) {2} from folder {1}")]
    AddDeletedFlagImapError(#[source] imap::Error, String, Id),
    #[error("cannot peek imap messages {2} from folder {1}")]
    PeekMessagesImapError(#[source] imap::Error, String, Id),
    #[cfg(feature = "notmuch")]
    #[error("cannot move notmuch message {3} from {1} to {2}")]
    MoveMessageNotmuchError(#[source] notmuch::Error, String, String, String),
    #[error("cannot move messages {3} from maildir folder {1} to folder {2}")]
    MoveMessagesMaildirError(#[source] maildirpp::Error, String, String, String),
    #[error("cannot move imap messages {3} from folder {1} to folder {2}")]
    MoveMessagesImapError(#[source] imap::Error, String, String, Id),
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
    AcountError(#[from] crate::account::error::Error),
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
    DecryptPartError(#[source] crate::account::error::Error),
    #[error("cannot interpret email as template")]
    InterpretEmailAsTplError(#[source] mml::Error),
    #[error("cannot parse email message")]
    ParseEmailMessageError,
    #[error("cannot get imap messages {2} from folder {1}")]
    GetMessagesImapError(#[source] imap::Error, String, Id),
    #[error("cannot get notmuch message filename from {0}")]
    GetMessageFilenameNotmuchError(PathBuf),
    #[cfg(feature = "notmuch")]
    #[error("cannot copy notmuch message {3} from {1} to {2}")]
    CopyMessageNotmuchError(#[source] notmuch::Error, String, String, String),
    #[error("cannot copy maildir messages {3} from folder {1} to folder {2}")]
    CopyMessagesMaildirError(#[source] maildirpp::Error, String, String, String),
    #[error("cannot copy imap messages {3} from folder {1} to folder {2}")]
    CopyMessagesImapError(#[source] imap::Error, String, String, Id),
    #[error("cannot add maildir message to folder {1} with flags {2}")]
    StoreWithFlagsMaildirError(#[source] maildirpp::Error, String, Flags),
    #[error("cannot add imap message to folder {1} with flags {2}")]
    AppendRawMessageWithFlagsImapError(#[source] imap::Error, String, Flags),
    #[error("cannot get added imap message uid from range {0}")]
    GetAddedMessageUidFromRangeImapError(String),
    #[error("cannot get added imap message uid: extension UIDPLUS may be missing on the server")]
    GetAddedMessageUidImapError,
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderMaildirError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderMaildirError(PathBuf, PathBuf),
    #[error("cannot create maildir {1} folder structure")]
    InitFolderMaildirError(#[source] maildirpp::Error, PathBuf),
    #[error("cannot examine imap folder {1}")]
    ExamineFolderImapError(#[source] imap::Error, String),
    #[error("cannot run imap idle mode")]
    RunIdleModeImapError(#[source] imap::Error),
    #[error("cannot list all imap envelopes of folder {1}")]
    ListAllEnvelopesImapError(#[source] imap::Error, String),
    #[error("cannot list notmuch envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsNotmuchError(String, usize),
    #[cfg(feature = "notmuch")]
    #[error("cannot list notmuch envelopes from {0}: invalid query {1}")]
    SearchMessagesInvalidQueryNotmuch(#[source] notmuch::Error, String, String),
    #[error("cannot list maildir envelopes from {0}: page {1} out of bounds")]
    GetEnvelopesOutOfBoundsMaildirError(String, usize),
    #[error("cannot list imap envelopes {2} from folder {1}")]
    ListEnvelopesImapError(#[source] imap::Error, String, String),
    #[error("cannot search imap envelopes from folder {1} with query {2}")]
    SearchEnvelopesImapError(#[source] imap::Error, String, String),
    #[error("cannot list imap envelopes: page {0} out of bounds")]
    BuildPageRangeOutOfBoundsImapError(usize),
    #[error("cannot get uid of imap envelope {0}: uid is missing")]
    GetUidMissingImapError(u32),
    #[error("cannot get missing envelope {0}")]
    GetEnvelopeMissingError(u32),
    #[error("cannot find notmuch envelope {1} from folder {0}")]
    FindEnvelopeEmptyNotmuchError(String, String),
    #[error("cannot find maildir envelope {1} from folder {0}")]
    GetEnvelopeMaildirError(PathBuf, Id),
    #[error("cannot fetch imap envelopes {2} from folder {1}")]
    FetchEnvolpesImapError(#[source] imap::Error, String, Id),
    #[error("cannot find imap envelope {1} from folder {0}")]
    GetFirstEnvelopeImapError(String, Id),
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagsMaildirError(#[source] maildirpp::Error, String, String, Flags),
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagImapError(#[source] imap::Error, String, Id, Flags),
    #[error("cannot remove flags {3} to envelope(s) {2} from folder {1}")]
    RemoveFlagsMaildirError(#[source] maildirpp::Error, String, String, Flags),
    #[error("cannot remove flags {3} to envelope(s) {2} from folder {1}")]
    RemoveFlagImapError(#[source] imap::Error, String, Id, Flags),
    #[error("cannot parse flag {0}")]
    ParseFlagError(String),
    #[error("cannot parse maildir flag char {0}")]
    ParseFlagMaildirError(char),
    #[error("cannot parse imap flag {0}")]
    ParseFlagImapError(String),
    #[error("cannot add maildir flags {3} to envelope(s) {2} from folder {1}")]
    AddFlagsMaildirError(#[source] maildirpp::Error, String, String, Flags),
    #[error("cannot select imap folder {1}")]
    SelectFolderImapError(#[source] imap::Error, String),
    #[error("cannot add imap flags {3} to envelope(s) {2} from folder {1}")]
    AddFlagImapError(#[source] imap::Error, String, Id, Flags),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("failed to get envelopes: {0}")]
    FailedToGetEnvelopes(JoinError),
    #[cfg(feature = "notmuch")]
    #[error("notmuch failed: {0}")]
    NotMuchFailure(notmuch::Error),
    #[error("process failed: {0}")]
    ProcessFailure(process::error::Error),
    #[error("maildir failed: {0}")]
    MaildirppFailure(maildirpp::Error),
    #[error("could not watch: {0}")]
    NotifyFailure(notify::Error),
    #[error("could not watch: {0}")]
    FileReadFailure(io::Error),
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
