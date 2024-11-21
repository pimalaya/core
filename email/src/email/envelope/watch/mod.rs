pub mod config;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "maildir")]
pub mod maildir;

use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::oneshot::{Receiver, Sender};
use tracing::{debug, info};

use crate::{account::config::AccountConfig, envelope::Envelope, AnyResult};

#[async_trait]
pub trait WatchEnvelopes: Send + Sync {
    /// Watch the given folder for envelopes changes.
    async fn watch_envelopes(
        &self,
        folder: &str,
        wait_for_shutdown_request: Receiver<()>,
        shutdown: Sender<()>,
    ) -> AnyResult<()>;

    async fn exec_hooks(
        &self,
        config: &AccountConfig,
        prev_envelopes: &HashMap<String, Envelope>,
        next_envelopes: &HashMap<String, Envelope>,
    ) {
        debug!("executing watch hooks…");
        for (id, envelope) in next_envelopes {
            // a new envelope has been added
            if !prev_envelopes.contains_key(id) {
                info!(id, "new message detected");
                debug!("processing received envelope event…");
                config.exec_received_envelope_hook(envelope).await;
            } else {
                // TODO
                // debug!("processing any envelope event…");
                // config.exec_any_envelope_hook(envelope).await;
            }
        }
    }
}
