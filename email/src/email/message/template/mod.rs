//! Module dedicated to email message templates.
//!
//! A template is a simplified version of an email MIME message, based
//! on [MML](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html).

mod forward;
mod new;
mod reply;

pub use pimalaya_email_tpl::{FilterParts, ShowHeadersStrategy, Tpl, TplInterpreter};
use thiserror::Error;

#[doc(inline)]
pub use self::{forward::ForwardTplBuilder, new::NewTplBuilder, reply::ReplyTplBuilder};

/// Errors related to email message templates.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot interpret message as template")]
    InterpretMessageAsTemplateError(#[source] pimalaya_email_tpl::tpl::interpreter::Error),
    #[error("cannot interpret message as thread template")]
    InterpretMessageAsThreadTemplateError(#[source] pimalaya_email_tpl::tpl::interpreter::Error),
}
