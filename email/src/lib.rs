//! Rust library to manage emails.
//!
//! The core concept of this library is to implement email actions and
//! to expose them into backend-agnostic abstractions. This way, you
//! can easily build email interfaces without caring about how to
//! connect to an IMAP server or how to send an email via SMTP.
//!
//! ## Backend features
//!
//! ### Folder
//!
//! - [`AddFolder`](crate::folder::AddFolder)
//! - [`ListFolders`](crate::folder::ListFolders)
//! - [`ExpungeFolder`](crate::folder::ExpungeFolder)
//! - [`PurgeFolder`](crate::folder::PurgeFolder)
//! - [`DeleteFolder`](crate::folder::DeleteFolder)
//!
//! ### Envelope
//!
//! - [`ListEnvelopes`](crate::email::envelope::ListEnvelopes)
//! - [`GetEnvelope`](crate::email::envelope::GetEnvelope)
//!
//! ### Flag
//!
//! - [`AddFlags`](crate::email::envelope::flag::AddFlags)
//!
//! ### Message
//!
//! - [`AddRawMessage`](crate::email::message::AddRawMessage)
//! - [`AddRawMessageWithFlags`](crate::email::message::AddRawMessageWithFlags) (implemented for `T: AddRawMessage + AddFlags`)
//! - [`PeekMessages`](crate::email::message::PeekMessages)
//! - [`GetMessages`](crate::email::message::GetMessages) (implemented for `T: PeekMessages + AddFlags`)
//! - [`CopyMessages`](crate::email::message::CopyMessages)
//!

pub mod account;
pub mod backend;
pub mod config;
pub mod email;
pub mod folder;
#[cfg(feature = "imap-backend")]
pub mod imap;
pub mod maildir;
#[cfg(feature = "notmuch-backend")]
pub mod notmuch;
pub mod sendmail;
#[cfg(feature = "smtp-sender")]
pub mod smtp;

/// The global `Error` alias of the library.
pub type Error = anyhow::Error;

/// The global `Result` alias of the library.
pub type Result<T> = anyhow::Result<T>;

pub mod prelude {
    pub use crate::{
        email::{
            envelope::{
                flag::{AddFlags, RemoveFlags, SetFlags},
                GetEnvelope, ListEnvelopes,
            },
            message::{
                AddRawMessageWithFlags, CopyMessages, DeleteMessages, GetMessages, MoveMessages,
                PeekMessages, SendRawMessage,
            },
        },
        folder::{AddFolder, DeleteFolder, ExpungeFolder, ListFolders, PurgeFolder},
    };
}
