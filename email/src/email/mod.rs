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
#[cfg(feature = "account-sync")]
pub mod sync;
pub mod utils;

#[doc(inline)]
pub use self::utils::*;
