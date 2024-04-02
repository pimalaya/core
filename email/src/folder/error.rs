use std::{io, path::PathBuf};

use thiserror::Error;
#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create imap folder {1}")]
    CreateFolderImapError(#[source] imap::Error, String),
    #[error("cannot create maildir folder structure at {1}")]
    CreateFolderStructureMaildirError(#[source] maildirpp::Error, PathBuf),
    #[error("cannot create notmuch folder structure at {1}")]
    CreateFolderStructureNotmuchError(#[source] maildirpp::Error, PathBuf),
    #[error("cannot delete imap folder {1}")]
    DeleteFolderImapError(#[source] imap::Error, String),
    #[error("cannot delete maildir folder {1}")]
    DeleteFolderMaildirError(#[source] io::Error, PathBuf),
    #[error("cannot select imap folder {1}")]
    SelectFolderImapError(#[source] imap::Error, String),
    #[error("cannot add imap flag deleted to all envelopes in folder {1}")]
    AddDeletedFlagImapError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderImapError(#[source] imap::Error, String),
    #[error("maildir: cannot list current folder from {1}")]
    ListCurrentFolderMaildirError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot delete message {2} from folder {1}")]
    DeleteMessageMaildirError(#[source] maildirpp::Error, PathBuf, String),
    #[error("cannot parse folder kind {0}")]
    ParseFolderKindError(String),
    #[error("cannot list imap folders")]
    ListFoldersImapError(#[source] imap::Error),
    #[error("cannot get uid of imap folder {0}: uid is missing")]
    GetUidMissingImapError(u32),
}
