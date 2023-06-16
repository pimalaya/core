//! Module dedicated to emails.
//!
//! An email is composed of two things:
//!
//! - The [envelope], which contains an identifier, some
//!   [flags](envelope::flag::Flags) and few headers.
//!
//! - The [message], which is the raw content of the
//!   email (header + body).
//!
//! This module also contains stuff related to email
//! [configuration](config) and [synchronization](sync).

pub mod config;
pub mod envelope;
pub mod message;
pub mod sync;
pub mod utils;

#[doc(inline)]
pub use self::{
    config::{EmailHooks, EmailTextPlainFormat},
    envelope::{
        flag::{self, Flag, Flags},
        Address, Envelope, Envelopes,
    },
    message::*,
    sync::{
        EmailSyncCache, EmailSyncCacheHunk, EmailSyncCachePatch, EmailSyncHunk, EmailSyncPatch,
        EmailSyncPatchManager, EmailSyncReport,
    },
    utils::*,
};
