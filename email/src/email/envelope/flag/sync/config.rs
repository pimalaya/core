#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct FlagSyncConfig {
    #[cfg_attr(feature = "derive", serde(default))]
    pub permissions: FlagSyncPermissions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct FlagSyncPermissions {
    #[cfg_attr(
        feature = "derive",
        serde(default = "FlagSyncPermissions::default_update")
    )]
    pub update: bool,
}

impl FlagSyncPermissions {
    pub fn default_update() -> bool {
        true
    }
}

impl Default for FlagSyncPermissions {
    fn default() -> Self {
        Self {
            update: Self::default_update(),
        }
    }
}
