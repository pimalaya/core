mod forward;
mod new;
mod reply;

use std::io;
use thiserror::Error;

#[doc(inline)]
pub use self::{forward::ForwardTplBuilder, new::NewTplBuilder, reply::ReplyTplBuilder};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot interpret email as template")]
    InterpretEmailAsTplError(#[source] pimalaya_email_tpl::tpl::interpreter::Error),
    #[error("cannot parse raw message")]
    ParseRawMessageError,
    #[error("cannot build forward template")]
    BuildForwardTplError(#[source] io::Error),
}
