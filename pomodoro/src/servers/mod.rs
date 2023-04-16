//! # Server binders module.
//!
//! This module contains server binders implementation packaged within
//! the lib. Every binder can be activated via cargo features.

#[cfg(feature = "tcp-binder")]
mod tcp;

#[cfg(feature = "tcp-binder")]
pub use tcp::*;
