//! # Clients module.
//!
//! This module contains clients implementation packaged within the
//! lib. Every client can be activated via cargo features.

#[cfg(feature = "pomodoro-tcp-client")]
mod tcp;

#[cfg(feature = "pomodoro-tcp-client")]
pub use tcp::*;
