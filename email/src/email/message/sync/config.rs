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
    #[cfg_attr(
        feature = "derive",
        serde(default = "MessageSyncPermissions::default_create")
    )]
    pub create: bool,

    #[cfg_attr(
        feature = "derive",
        serde(default = "MessageSyncPermissions::default_delete")
    )]
    pub delete: bool,
}

impl MessageSyncPermissions {
    pub fn default_create() -> bool {
        true
    }

    pub fn default_delete() -> bool {
        true
    }
}

impl Default for MessageSyncPermissions {
    fn default() -> Self {
        Self {
            create: Self::default_create(),
            delete: Self::default_delete(),
        }
    }
}
