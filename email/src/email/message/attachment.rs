//! Module dedicated to email message attachment.
//!
//! This module contains everything related to email message
//! attachments.

/// The email message attachment.
///
/// Represents a simplified version of an email message attachment.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Attachment {
    /// The optional attachment filename.
    pub filename: Option<String>,

    /// The attachment MIME type.
    pub mime: String,

    /// The raw content of the attachment.
    pub body: Vec<u8>,
}
