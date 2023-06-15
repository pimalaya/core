//! Message module.
//!
//! This module contains everything related to emails.

pub(crate) mod address;
pub mod config;
pub mod envelope;
pub mod message;
pub mod utils;

#[doc(inline)]
pub use self::{
    config::{EmailHooks, EmailTextPlainFormat},
    envelope::{
        Envelope, EnvelopeSyncCache, EnvelopeSyncCacheHunk, EnvelopeSyncCachePatch,
        EnvelopeSyncHunk, EnvelopeSyncPatch, EnvelopeSyncPatchManager, Envelopes,
    },
    message::*,
    utils::*,
};
