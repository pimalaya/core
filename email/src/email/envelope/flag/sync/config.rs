use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FlagSyncConfig {
    #[serde(default)]
    pub permissions: FlagSyncPermissions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FlagSyncPermissions {
    #[serde(default)]
    pub update: bool,
}

impl Default for FlagSyncPermissions {
    fn default() -> Self {
        Self { update: true }
    }
}
