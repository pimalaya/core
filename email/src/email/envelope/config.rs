use serde::{Deserialize, Serialize};

use super::{list::config::EnvelopeListConfig, watch::config::WatchEnvelopeConfig};

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EnvelopeConfig {
    /// The envelope config related to listing.
    pub list: Option<EnvelopeListConfig>,

    /// Configuration dedicated to envelope changes.
    pub watch: Option<WatchEnvelopeConfig>,
}
