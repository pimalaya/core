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
#[cfg(feature = "autoconfig")]
pub mod autoconfig;
pub mod backend;
pub mod config;
pub mod email;
mod error;
pub mod folder;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;
#[cfg(feature = "notmuch")]
pub mod notmuch;
pub mod retry;
#[cfg(feature = "sendmail")]
pub mod sendmail;
#[cfg(feature = "derive")]
pub(crate) mod serde;
#[cfg(feature = "smtp")]
pub mod smtp;
#[cfg(feature = "sync")]
pub mod sync;
#[cfg(any(feature = "imap", feature = "smtp"))]
pub mod tls;
pub mod watch;

#[doc(inline)]
pub use crate::{
    email::{envelope::flag, message::template, *},
    error::{AnyBoxedError, AnyError, AnyResult},
};
