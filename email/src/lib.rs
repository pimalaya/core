//! Rust library to manage emails.
//!
//! The main purpose of this library is to help people to build custom
//! email interfaces without caring about how to connect to an IMAP
//! server or how to send an email via SMTP.
//!
//! This goal is achieved by exposing a
//! [`Backend`](crate::backend::Backend) struct which is just a set of
//! customizable features like adding a folder, listing envelopes or
//! sending a message. You also have access to a
//! [`BackendBuilder`](crate::backend::BackendBuilder) which helps to
//! build a custom backend.
//!
//! The library also exposes pre-configured backend features for
//! Maildir, IMAP, Notmuch, SMTP and Sendmail.
//!
//! See examples in the `/tests` folder.
//!
//! ## Backend features
//!
//! ### Folder
//!
//! - [`AddFolder`](crate::folder::add::AddFolder)
//! - [`ListFolders`](crate::folder::list::ListFolders)
//! - [`ExpungeFolder`](crate::folder::expunge::ExpungeFolder)
//! - [`PurgeFolder`](crate::folder::purge::PurgeFolder)
//! - [`DeleteFolder`](crate::folder::delete::DeleteFolder)
//!
//! ### Envelope
//!
//! - [`ListEnvelopes`](crate::envelope::list::ListEnvelopes)
//! - [`GetEnvelope`](crate::envelope::get::GetEnvelope)
//!
//! ### Flag
//!
//! - [`AddFlags`](crate::flag::add::AddFlags)
//! - [`SetFlags`](crate::flag::set::SetFlags)
//! - [`RemoveFlags`](crate::flag::remove::RemoveFlags)
//!
//! ### Message
//!
//! - [`AddRawMessage`](crate::message::add_raw::AddRawMessage)
//! - [`AddRawMessageWithFlags`](crate::message::add_raw_with_flags::AddRawMessageWithFlags)
//! - [`PeekMessages`](crate::message::peek::PeekMessages)
//! - [`GetMessages`](crate::message::get::GetMessages)
//! - [`CopyMessages`](crate::message::copy::CopyMessages)
//! - [`MoveMessages`](crate::message::move_::MoveMessages)
//! - [`DeleteMessages`](crate::message::delete::DeleteMessages)
//! - [`SendRawMessage`](crate::message::send_raw::SendRawMessage)

pub mod account;
pub mod backend;
pub mod config;
pub mod email;
pub mod folder;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;
#[cfg(feature = "sendmail")]
pub mod sendmail;
#[cfg(feature = "smtp")]
pub mod smtp;
#[cfg(feature = "sync")]
pub mod sync;
pub mod thread_pool;
pub mod watch;

#[doc(inline)]
pub use email::{
    envelope::{self, flag},
    message::{self, template},
};

/// The global `Error` alias of the library.
///
/// Downcasting should suffice in most cases; since usecases for precise
/// error variant identification in `email-lib` should be rare.
/// While suitable for most libraries, using one error per module in
/// a large library like `email-lib` complicates communication due to
/// differences in errors.
pub type Error = anyhow::Error;

/// The global `Result` alias of the library.
///
/// Refer to the `Error` documentation for an explanation
/// about the choice of using `anyhow` crate on the library level.
pub type Result<T> = anyhow::Result<T>;
