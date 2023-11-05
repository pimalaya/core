//! Module dedicated to email management.
//!
//! An email is composed of two things:
//!
//! - The **envelope**, which contains an identifier, some flags and
//! few headers.
//!
//! - The **message**, which is the raw content of the email (header +
//! body).
//!
//! This module also contains stuff related to email configuration and
//! synchronization.

pub mod config;
pub mod envelope;
pub mod message;
pub mod sync;
pub mod utils;

#[doc(inline)]
pub use mail_builder::MessageBuilder;

#[doc(inline)]
pub use self::{
    config::{EmailHooks, EmailTextPlainFormat},
    envelope::{
        flag::{self, Flag, Flags},
        Envelope, Envelopes,
    },
    message::{
        attachment::{self, Attachment},
        template, Message, Messages,
    },
    utils::*,
};
