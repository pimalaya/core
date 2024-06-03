use super::list::config::EnvelopeListConfig;
#[cfg(feature = "sync")]
use super::sync::config::EnvelopeSyncConfig;
#[cfg(feature = "thread")]
use super::thread::config::EnvelopeThreadConfig;
#[cfg(feature = "watch")]
use super::watch::config::WatchEnvelopeConfig;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct EnvelopeConfig {
    /// The envelope config related to listing.
    pub list: Option<EnvelopeListConfig>,

    /// The envelope config related to threading.
    #[cfg(feature = "thread")]
    pub thread: Option<EnvelopeThreadConfig>,

    /// Configuration dedicated to envelope changes.
    #[cfg(feature = "watch")]
    pub watch: Option<WatchEnvelopeConfig>,

    /// Configuration dedicated to envelope changes.
    #[cfg(feature = "sync")]
    pub sync: Option<EnvelopeSyncConfig>,
}
