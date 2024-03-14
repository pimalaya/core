use crate::watch::config::WatchHook;

/// Configuration dedicated to envelope changes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct WatchEnvelopeConfig {
    /// Watch hook configuration for when a new envelope has been
    /// received.
    pub received: Option<WatchHook>,

    /// Watch hook configuration hook for any other case.
    pub any: Option<WatchHook>,
}
