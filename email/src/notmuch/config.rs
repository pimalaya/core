//! Module dedicated to the Notmuch backend configuration.
//!
//! This module contains the configuration specific to the Notmuch
//! backend.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// The Notmuch backend config.
#[derive(Debug, Default, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NotmuchConfig {
    /// The path to the Notmuch database.
    ///
    /// The path should point to the root directory containing the
    /// Notmuch database (usually the root Maildir directory). Path is
    /// shell-expanded, which means environment variables and tilde
    /// `~` are replaced by their values.
    pub db_path: PathBuf,
}
