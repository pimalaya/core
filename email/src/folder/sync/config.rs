//! # Folder sync config

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FolderSyncConfig {
    #[serde(default)]
    pub filter: FolderSyncStrategy,

    #[serde(default)]
    pub permissions: FolderSyncPermissions,
}

/// The folder synchronization strategy.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FolderSyncStrategy {
    /// Synchronizes all folders.
    #[default]
    All,

    /// Synchronizes only folders matching the given names.
    Include(BTreeSet<String>),

    /// Synchronizes all folders except the ones matching the given
    /// names.
    Exclude(BTreeSet<String>),
}

impl FolderSyncStrategy {
    pub fn matches(&self, folder: &str) -> bool {
        match self {
            FolderSyncStrategy::All => true,
            FolderSyncStrategy::Include(folders) => folders.contains(folder),
            FolderSyncStrategy::Exclude(folders) => !folders.contains(folder),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FolderSyncPermissions {
    #[serde(default)]
    pub create: bool,

    #[serde(default)]
    pub delete: bool,
}

impl Default for FolderSyncPermissions {
    fn default() -> Self {
        Self {
            create: true,
            delete: true,
        }
    }
}
