//! # Server binders module.
//!
//! This module contains server binders implementation packaged within
//! the lib. Every binder can be activated via cargo features.

#[cfg(feature = "pomodoro-tcp-binder")]
mod tcp;

#[cfg(feature = "pomodoro-tcp-binder")]
pub use tcp::*;
