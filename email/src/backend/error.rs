use std::{any::Any, result};

use thiserror::Error;

use crate::{AnyBoxedError, AnyError};

/// The global `Result` alias of the module.
pub type Result<T> = result::Result<T, Error>;

/// The global `Error` enum of the module.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot add folder: feature not available, or backend configuration for this functionality is not set")]
    AddFolderNotAvailableError,
    #[error("cannot list folders: feature not available, or backend configuration for this functionality is not set")]
    ListFoldersNotAvailableError,
    #[error("cannot expunge folder: feature not available, or backend configuration for this functionality is not set")]
    ExpungeFolderNotAvailableError,
    #[error("cannot purge folder: feature not available, or backend configuration for this functionality is not set")]
    PurgeFolderNotAvailableError,
    #[error("cannot delete folder: feature not available, or backend configuration for this functionality is not set")]
    DeleteFolderNotAvailableError,
    #[error("cannot list envelopes: feature not available, or backend configuration for this functionality is not set")]
    ListEnvelopesNotAvailableError,
    #[error("cannot thread envelopes: feature not available, or backend configuration for this functionality is not set")]
    ThreadEnvelopesNotAvailableError,
    #[error("cannot watch for envelopes changes: feature not available, or backend configuration for this functionality is not set")]
    WatchEnvelopesNotAvailableError,
    #[error("cannot get envelope: feature not available, or backend configuration for this functionality is not set")]
    GetEnvelopeNotAvailableError,
    #[error("cannot add flag(s): feature not available, or backend configuration for this functionality is not set")]
    AddFlagsNotAvailableError,
    #[error("cannot set flag(s): feature not available, or backend configuration for this functionality is not set")]
    SetFlagsNotAvailableError,
    #[error("cannot remove flag(s): feature not available, or backend configuration for this functionality is not set")]
    RemoveFlagsNotAvailableError,
    #[error("cannot add message: feature not available, or backend configuration for this functionality is not set")]
    AddMessageNotAvailableError,
    #[error("cannot add message with flags: feature not available, or backend configuration for this functionality is not set")]
    AddMessageWithFlagsNotAvailableError,
    #[error("cannot send message: feature not available, or backend configuration for this functionality is not set")]
    SendMessageNotAvailableError,
    #[error("cannot get messages: feature not available, or backend configuration for this functionality is not set")]
    GetMessagesNotAvailableError,
    #[error("cannot peek messages: feature not available, or backend configuration for this functionality is not set")]
    PeekMessagesNotAvailableError,
    #[error("cannot copy messages: feature not available, or backend configuration for this functionality is not set")]
    CopyMessagesNotAvailableError,
    #[error("cannot move messages: feature not available, or backend configuration for this functionality is not set")]
    MoveMessagesNotAvailableError,
    #[error("cannot delete messages: feature not available, or backend configuration for this functionality is not set")]
    DeleteMessagesNotAvailableError,
    #[error("cannot remove messages: feature not available, or backend configuration for this functionality is not set")]
    RemoveMessagesNotAvailableError,
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
