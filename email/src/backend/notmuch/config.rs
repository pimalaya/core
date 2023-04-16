//! Notmuch backend config module.
//!
//! This module contains the representation of the notmuch backend
//! configuration of the user account.

use std::path::PathBuf;

/// Represents the Notmuch backend config.
#[cfg(feature = "notmuch-backend")]
#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct NotmuchConfig {
    /// Represents the notmuch database path.
    pub db_path: PathBuf,
}
