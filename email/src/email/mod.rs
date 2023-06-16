//! Module dedicated to emails.

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
