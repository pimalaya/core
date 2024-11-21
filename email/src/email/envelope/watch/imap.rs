use std::collections::HashMap;

use async_trait::async_trait;
use tokio::sync::oneshot::{Receiver, Sender};
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::WatchEnvelopes;
use crate::{envelope::Envelope, imap::ImapContext, AnyResult};

#[derive(Clone, Debug)]
pub struct WatchImapEnvelopes {
    ctx: ImapContext,
}

impl WatchImapEnvelopes {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn WatchEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn WatchEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }

    pub async fn watch_envelopes_loop(
        &self,
        folder: &str,
        wait_for_shutdown_request: &mut Receiver<()>,
    ) -> AnyResult<()> {
        info!("watching imap folder {folder} for envelope changes");

        let config = &self.ctx.account_config;
        let mut client = self.ctx.client().await;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let envelopes_count = client
            .examine_mailbox(folder_encoded)
            .await?
            .exists
            .unwrap() as usize;

        let envelopes = if envelopes_count == 0 {
            Default::default()
        } else {
            client.fetch_all_envelopes().await?
        };

        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        loop {
            info!("starting new IMAP IDLE loop…");
            client.idle(wait_for_shutdown_request).await?;
            info!("received IDLE change notification or timeout");

            let next_envelopes = client.fetch_all_envelopes().await?;
            let next_envelopes: HashMap<String, Envelope> =
                HashMap::from_iter(next_envelopes.into_iter().map(|e| (e.id.clone(), e)));

            self.exec_hooks(config, &envelopes, &next_envelopes).await;

            envelopes = next_envelopes;
        }
    }
}

#[async_trait]
impl WatchEnvelopes for WatchImapEnvelopes {
    async fn watch_envelopes(
        &self,
        folder: &str,
        mut wait_for_shutdown_request: Receiver<()>,
        shutdown: Sender<()>,
    ) -> AnyResult<()> {
        let res = self
            .watch_envelopes_loop(folder, &mut wait_for_shutdown_request)
            .await;

        shutdown.send(()).unwrap();

        res
    }
}
