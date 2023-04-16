//! # Clients module.
//!
//! This module contains clients implementation packaged within the
//! lib. Every client can be activated via cargo features.

#[cfg(feature = "tcp-client")]
mod tcp;

#[cfg(feature = "tcp-client")]
pub use tcp::*;
