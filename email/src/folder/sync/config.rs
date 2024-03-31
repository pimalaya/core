//! # Folder sync config

use std::collections::BTreeSet;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct FolderSyncConfig {
    #[cfg_attr(feature = "derive", serde(default))]
    pub filter: FolderSyncStrategy,

    #[cfg_attr(feature = "derive", serde(default))]
    pub permissions: FolderSyncPermissions,
}

/// The folder synchronization strategy.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct FolderSyncPermissions {
    #[cfg_attr(
        feature = "derive",
        serde(default = "FolderSyncPermissions::default_create")
    )]
    pub create: bool,

    #[cfg_attr(
        feature = "derive",
        serde(default = "FolderSyncPermissions::default_delete")
    )]
    pub delete: bool,
}

impl FolderSyncPermissions {
    pub fn default_create() -> bool {
        true
    }

    pub fn default_delete() -> bool {
        true
    }
}

impl Default for FolderSyncPermissions {
    fn default() -> Self {
        Self {
            create: Self::default_create(),
            delete: Self::default_delete(),
        }
    }
}
