//! Module dedicated to email message templates.
//!
//! A template is a simplified version of an email MIME message, based
//! on [MML](https://www.gnu.org/software/emacs/manual/html_node/emacs-mime/Composing.html).

pub mod config;
pub mod forward;
pub mod new;
pub mod reply;

pub use mml::{
    message::{FilterHeaders, FilterParts},
    MimeInterpreter,
};
use thiserror::Error;

/// Errors related to email message templates.
#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot interpret message as template")]
    InterpretMessageAsTemplateError(#[source] mml::Error),
    #[error("cannot interpret message as thread template")]
    InterpretMessageAsThreadTemplateError(#[source] mml::Error),
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct Template {
    pub content: String,
    pub row: usize,
    pub col: usize,
}

impl Template {
    pub fn new(content: impl ToString) -> Self {
        Self::new_with_cursor(content, 0, 0)
    }

    pub fn new_with_cursor(content: impl ToString, row: usize, col: usize) -> Self {
        let content = content.to_string();
        Self { content, row, col }
    }
}
