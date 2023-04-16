//! Maildir backend config module.
//!
//! This module contains the representation of the Maildir backend
//! configuration of the user account.

use std::path::PathBuf;

/// Represents the Maildir backend config.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct MaildirConfig {
    /// Represents the Maildir root directory.
    pub root_dir: PathBuf,
}
