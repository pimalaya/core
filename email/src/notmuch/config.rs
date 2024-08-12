//! Module dedicated to the Notmuch backend configuration.
//!
//! This module contains the configuration specific to the Notmuch
//! backend.

use std::path::{Path, PathBuf};

use notmuch::{Database, DatabaseMode};
use shellexpand_utils::shellexpand_path;

#[doc(inline)]
pub use super::{Error, Result};

/// The Notmuch backend config.
#[derive(Debug, Default, Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct NotmuchConfig {
    /// The path to the Notmuch database.
    ///
    /// The path should point to the root directory containing the
    /// Notmuch database (usually the root Maildir directory). Path is
    /// shell-expanded, which means environment variables and tilde
    /// `~` are replaced by their values.
    #[cfg_attr(feature = "derive", serde(alias = "db-path"))]
    pub database_path: Option<PathBuf>,

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

    #[cfg_attr(feature = "derive", serde(default))]
    pub maildirpp: bool,
}

impl NotmuchConfig {
    /// Get the default Notmuch database path.
    pub fn get_default_database_path() -> Result<PathBuf> {
        Ok(Database::open_with_config(
            None::<PathBuf>,
            DatabaseMode::ReadOnly,
            None::<PathBuf>,
            None,
        )
        .map_err(Error::OpenDatabaseError)?
        .path()
        .to_owned())
    }

    /// Try to get the reference to the Notmuch database path.
    pub fn try_get_database_path(&self) -> Result<PathBuf> {
        match self.database_path.as_ref() {
            Some(path) => Ok(shellexpand_path(path)),
            None => Self::get_default_database_path(),
        }
    }

    /// Try to get the reference to the Maildir path.
    ///
    /// Tries `maildir_path` first, otherwise falls back to
    /// `database_path`.
    pub fn try_get_maildir_path(&self) -> Result<PathBuf> {
        match self.maildir_path.as_ref() {
            Some(path) => Ok(shellexpand_path(path)),
            None => self.try_get_database_path(),
        }
    }

    /// Find the Notmuch configuration path reference.
    pub fn find_config_path(&self) -> Option<&Path> {
        self.config_path.as_ref().map(AsRef::as_ref)
    }

    /// Find the Notmuch profile.
    pub fn find_profile(&self) -> Option<&str> {
        self.profile.as_deref()
    }
}
