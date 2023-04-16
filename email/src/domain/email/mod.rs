//! Message module.
//!
//! This module contains everything related to emails.

pub mod attachment;
pub mod config;
pub mod email;
pub mod utils;

pub use attachment::Attachment;
pub use config::{EmailHooks, EmailSender, EmailTextPlainFormat};
pub use email::*;
pub use utils::*;
