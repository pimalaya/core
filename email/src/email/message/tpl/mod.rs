mod forward;
mod new;
mod reply;

use std::{io, result};

#[doc(inline)]
pub use forward::ForwardTplBuilder;
#[doc(inline)]
pub use new::NewTplBuilder;
#[doc(inline)]
pub use reply::ReplyTplBuilder;

use thiserror::Error;

use crate::{account, message};

#[derive(Debug, Error)]
pub enum Error {
    // #[error("cannot parse email")]
    // GetMailEntryError(#[source] maildirpp::Error),

    // #[error("cannot get parsed version of email: {0}")]
    // GetParsedEmailError(String),
    // #[error("cannot parse email")]
    // ParseEmailError,
    // #[error("cannot parse email: raw email is empty")]
    // ParseEmailEmptyRawError,
    // #[error("cannot delete local draft at {1}")]
    // DeleteLocalDraftError(#[source] io::Error, PathBuf),

    // #[error("cannot parse email: empty entries")]
    // ParseEmailFromEmptyEntriesError,
    #[error(transparent)]
    ConfigError(#[from] account::config::Error),
    #[error(transparent)]
    MessageError(#[from] Box<message::Error>),
    // #[error("cannot decrypt encrypted email part")]
    // DecryptEmailPartError(#[source] pimalaya_process::Error),
    // #[error("cannot verify signed email part")]
    // VerifyEmailPartError(#[source] pimalaya_process::Error),

    // // TODO: sort me
    // #[error("cannot get content type of multipart")]
    // GetMultipartContentTypeError,
    // #[error("cannot find encrypted part of multipart")]
    // GetEncryptedPartMultipartError,
    // #[error("cannot parse encrypted part of multipart")]
    // WriteEncryptedPartBodyError(#[source] io::Error),
    // #[error("cannot write encrypted part to temporary file")]
    // DecryptPartError(#[source] account::config::Error),
    #[error("cannot interpret email as template")]
    InterpretEmailAsTplError(#[source] pimalaya_email_tpl::tpl::interpreter::Error),
    #[error("cannot parse raw message")]
    ParseRawMessageError,
    #[error("cannot build forward template")]
    BuildForwardTplError(#[source] io::Error),
}

pub type Result<T> = result::Result<T, Error>;
