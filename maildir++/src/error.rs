use std::{io, path::PathBuf, result, time};

use thiserror::Error;
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find email {0}")]
    FindEmailError(String),
    #[error("cannot get invalid file name from email {0}")]
    GetEmailFileNameError(String),
    #[error("cannot copy email to the same path {0}")]
    CopyEmailSamePathError(PathBuf),
    #[error("cannot get subfolder name")]
    GetSubfolderNameError,

    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error(transparent)]
    SystemTimeError(#[from] time::SystemTimeError),
}

pub type Result<T> = result::Result<T, Error>;
