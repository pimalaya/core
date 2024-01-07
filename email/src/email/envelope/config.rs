use serde::{Deserialize, Serialize};

#[cfg(feature = "envelope-list")]
use super::list::config::EnvelopeListConfig;
#[cfg(feature = "envelope-watch")]
use super::watch::config::WatchEnvelopeConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvelopeConfig {
    #[cfg(feature = "envelope-list")]
    /// The envelope config related to listing.
    pub list: Option<EnvelopeListConfig>,

    #[cfg(feature = "envelope-watch")]
    /// Configuration dedicated to envelope changes.
    pub watch: Option<WatchEnvelopeConfig>,
}
