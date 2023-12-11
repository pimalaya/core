use serde::{Deserialize, Serialize};

use super::list::config::EnvelopeListConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeConfig {
    /// The envelope config related to listing.
    pub list: Option<EnvelopeListConfig>,
}
