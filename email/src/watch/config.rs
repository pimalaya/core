use std::{fmt, future::Future, ops::Deref, pin::Pin, sync::Arc};

use process::Command;

use crate::envelope::Envelope;

/// Watch hook configuration.
///
/// Each variant represent the action that should be done when a
/// change occurs.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub struct WatchHook {
    /// Execute the shell command.
    ///
    /// For now, command is executed without any parameter nor
    /// input. This may change in the future.
    pub cmd: Option<Command>,

    /// Send a system notification using the given
    /// [`notify_rust::Notification`]-like configuration.
    pub notify: Option<WatchNotifyConfig>,

    /// Execute the given watch function.
    ///
    /// The watch function cannot be de/serialized. The function
    /// should take a reference to an envelope and return a [`Result`]
    /// of unit.
    #[cfg_attr(feature = "derive", serde(skip))]
    pub callback: Option<WatchFn>,
}

impl Eq for WatchHook {
    //
}

impl PartialEq for WatchHook {
    fn eq(&self, other: &Self) -> bool {
        self.cmd == other.cmd && self.notify == other.notify
    }
}

/// Watch function.
///
/// This is just a wrapper around a function that takes a reference to
/// an envelope.
#[derive(Clone)]
pub struct WatchFn(
    // This function is essentially an asyc Fn with an empty anyhow result type.
    // So it should not create too much type complexity.
    // Added to that there is a new function that can take the complexity of
    // pinning and arcing away.
    #[allow(clippy::type_complexity)]
    Arc<
        dyn Fn(&Envelope) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> + Send + Sync,
    >,
);

impl WatchFn {
    /// Create a new watch function.
    pub fn new<F: Future<Output = crate::Result<()>> + Send + 'static>(
        f: impl Fn(&Envelope) -> F + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(move |envelope| Box::pin(f(envelope))))
    }
}

impl Default for WatchFn {
    fn default() -> Self {
        Self(Arc::new(|_| Box::pin(async { Ok(()) })))
    }
}

impl Deref for WatchFn {
    type Target = Arc<
        dyn Fn(&Envelope) -> Pin<Box<dyn Future<Output = crate::Result<()>> + Send>> + Send + Sync,
    >;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Debug for WatchFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WatchFn()")
    }
}

/// The watch configuration of the notify hook variant.
///
/// The structure tries to match the [`notify_rust::Notification`] API
/// and may evolve in the future.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(
    feature = "derive",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
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
