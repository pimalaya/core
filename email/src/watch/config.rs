use process::Cmd;
use serde::{Deserialize, Serialize};
use std::{fmt, ops::Deref, sync::Arc};

use crate::{envelope::Envelope, Result};

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

    /// Send a system notification using the given
    /// [`notify_rust::Notification`]-like configuration.
    Notify(WatchNotifyConfig),

    /// Execute the given watch function.
    ///
    /// The watch function cannot be de/serialized. The function
    /// should take a reference to an envelope and return a [`Result`]
    /// of unit.
    #[serde(skip)]
    Fn(WatchFn),
}

/// Watch function.
///
/// This is just a wrapper around a function that takes a reference to
/// an envelope.
#[derive(Clone)]
pub struct WatchFn(Arc<dyn Fn(&Envelope) -> Result<()> + Send + Sync>);

impl WatchFn {
    /// Create a new watch function.
    pub fn new(f: impl Fn(&Envelope) -> Result<()> + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
}

impl Deref for WatchFn {
    type Target = Arc<dyn Fn(&Envelope) -> Result<()> + Send + Sync>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for WatchFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WatchHookFn()")
    }
}

impl Eq for WatchFn {
    //
}

impl PartialEq for WatchFn {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
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
    ///  - "{sender}" either the sender name or the address
    ///  - "{sender.name}" the sender name or "unknown"
    ///  - "{sender.address}" the sender address
    ///  - "{recipient}" either the recipient name or the address
    ///  - "{recipient.name}" the recipient name or "unknown"
    ///  - "{recipient.address}" the recipient address
    pub summary: String,

    /// The body of the notification.
    ///
    /// Accepted placeholders:
    ///  - "{id}": the id of the envelope
    ///  - "{subject}": the subject of the envelope
    ///  - "{sender}" either the sender name or the address
    ///  - "{sender.name}" the sender name or "unknown"
    ///  - "{sender.address}" the sender address
    ///  - "{recipient}" either the recipient name or the address
    ///  - "{recipient.name}" the recipient name or "unknown"
    ///  - "{recipient.address}" the recipient address
    pub body: String,
}
