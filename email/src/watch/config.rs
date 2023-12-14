use process::Cmd;
use serde::{Deserialize, Serialize};

/// Watch hook configuration.
///
/// Each variant represent the action that should be done when a
/// change occurs.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WatchHook {
    /// Execute the shell command.
    ///
    /// For now, command is executed without any parameter nor
    /// input. This may change in the future.
    Cmd(Cmd),

    /// Send a system notification
    Notify(WatchNotifyConfig),
}

/// The watch configuration of the notify hook variant.
///
/// The structure tries to match the [`notify_rust::Notification`] API
/// and may evolve in the future.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WatchNotifyConfig {
    /// The summary (or the title) of the notification.
    ///
    /// Accepted placeholders:
    ///  - "{id}": the id of the envelope
    ///  - "{subject}": the subject of the envelope
    ///  - "{sender}" either the sender name or its address
    ///  - "{recipient}" either the recipient name or its address
    pub summary: String,

    /// The body of the notification.
    ///
    /// Accepted placeholders:
    ///  - "{id}": the id of the envelope
    ///  - "{subject}": the subject of the envelope
    ///  - "{sender}" either the sender name or its address
    ///  - "{recipient}" either the recipient name or its address
    pub body: String,
}
