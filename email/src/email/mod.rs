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
pub mod date;
pub mod envelope;
mod error;
pub mod message;
pub mod search_query;
#[cfg(feature = "sync")]
pub mod sync;
pub mod utils;

#[cfg(feature = "sync")]
pub(crate) use sync::sync;
#[doc(inline)]
pub use {
    self::utils::*,
    error::{Error, Result},
};
