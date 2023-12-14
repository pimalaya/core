use serde::{Deserialize, Serialize};

use crate::watch::config::WatchHook;

/// Configuration dedicated to message changes.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WatchMessageConfig {
    /// Watch hook configuration for when a new message has been
    /// received.
    pub received: Option<WatchHook>,

    /// Watch hook configuration hook for any other cases.
    pub any: Option<WatchHook>,
}
