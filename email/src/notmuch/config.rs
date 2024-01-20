//! Module dedicated to the Notmuch backend configuration.
//!
//! This module contains the configuration specific to the Notmuch
//! backend.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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
    #[serde(alias = "db-path")]
    pub database_path: PathBuf,

    /// Override the default path to the Maildir folder.
    ///
    /// Path is shell-expanded, which means environment variables and
    /// tilde `~` are replaced by their values. Defaults to
    /// `database_path` if omitted.
    pub maildir_path: Option<PathBuf>,

    /// Override the default Notmuch configuration file path.
    ///
    /// Path is shell-expanded, which means environment variables and
    /// tilde `~` are replaced by their values.
    pub config_path: Option<PathBuf>,

    /// Override the default Notmuch profile name.
    pub profile: Option<String>,
}

impl NotmuchConfig {
    /// Get the reference to the Notmuch database path.
    pub fn get_database_path(&self) -> &Path {
        self.database_path.as_ref()
    }

    /// Get the reference to the Maildir path.
    ///
    /// Try the `maildir_path` first, otherwise falls back to
    /// `database_path`.
    pub fn get_maildir_path(&self) -> &Path {
        self.maildir_path
            .as_ref()
            .unwrap_or(&self.database_path)
            .as_ref()
    }

    /// Find the Notmuch configuration path reference.
    pub fn find_config_path(&self) -> Option<&Path> {
        self.config_path.as_ref().map(AsRef::as_ref)
    }

    /// Find the Notmuch profile.
    pub fn find_profile(&self) -> Option<&str> {
        self.profile.as_ref().map(String::as_str)
    }
}
