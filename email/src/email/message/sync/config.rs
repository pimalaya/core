#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct MessageSyncConfig {
    #[cfg_attr(feature = "derive", serde(default))]
    pub permissions: MessageSyncPermissions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct MessageSyncPermissions {
    #[cfg_attr(feature = "derive", serde(default))]
    pub create: bool,

    #[cfg_attr(feature = "derive", serde(default))]
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
