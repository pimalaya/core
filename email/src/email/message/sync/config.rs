use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageSyncConfig {
    #[serde(default)]
    pub permissions: MessageSyncPermissions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MessageSyncPermissions {
    #[serde(default)]
    pub create: bool,

    #[serde(default)]
    pub delete: bool,
}

impl Default for MessageSyncPermissions {
    fn default() -> Self {
        Self {
            create: true,
            delete: true,
        }
    }
}
