#[cfg(feature = "account-sync")]
use super::sync::config::EnvelopeSyncConfig;
use super::{list::config::EnvelopeListConfig, watch::config::WatchEnvelopeConfig};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeConfig {
    /// The envelope config related to listing.
    pub list: Option<EnvelopeListConfig>,

    /// Configuration dedicated to envelope changes.
    pub watch: Option<WatchEnvelopeConfig>,

    #[cfg(feature = "account-sync")]
    /// Configuration dedicated to envelope changes.
    pub sync: Option<EnvelopeSyncConfig>,
}
