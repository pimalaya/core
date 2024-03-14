#[cfg(feature = "account-sync")]
use super::sync::config::FlagSyncConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct FlagConfig {
    #[cfg(feature = "account-sync")]
    /// Configuration dedicated to flag synchronization.
    pub sync: Option<FlagSyncConfig>,
}
