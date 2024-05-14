use std::collections::HashMap;

use async_ctrlc::CtrlC;
use async_trait::async_trait;
use tokio::sync::oneshot;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::WatchEnvelopes;
use crate::{debug, envelope::Envelope, imap::ImapContextSync, info, AnyResult};

#[derive(Clone, Debug)]
pub struct WatchImapEnvelopes {
    ctx: ImapContextSync,
}

impl WatchImapEnvelopes {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn WatchEnvelopes> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn WatchEnvelopes>> {
        Some(Self::new_boxed(ctx))
    }

    pub async fn watch_envelopes_loop(
        &self,
        folder: &str,
        wait_for_shutdown_request: &mut oneshot::Receiver<()>,
    ) -> AnyResult<()> {
        info!("watching imap folder {folder} for envelope changes");

        let config = &self.ctx.account_config;
        let mut ctx = self.ctx.lock().await;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let envelopes_count = ctx.examine_mailbox(folder_encoded).await?.exists.unwrap() as usize;

        let envelopes = if envelopes_count == 0 {
            Default::default()
        } else {
            ctx.fetch_all_envelopes().await?
        };

        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        loop {
            ctx.idle(wait_for_shutdown_request).await?;

            let next_envelopes = ctx.fetch_all_envelopes().await?;
            let next_envelopes: HashMap<String, Envelope> =
                HashMap::from_iter(next_envelopes.into_iter().map(|e| (e.id.clone(), e)));

            self.exec_hooks(config, &envelopes, &next_envelopes).await;

            envelopes = next_envelopes;
        }
    }
}

#[async_trait]
impl WatchEnvelopes for WatchImapEnvelopes {
    async fn watch_envelopes(&self, folder: &str) -> AnyResult<()> {
        info!("watching imap folder {folder} for envelope changes");

        let (request_shutdown, mut wait_for_shutdown_request) = oneshot::channel();
        let (shutdown, wait_for_shutdown) = oneshot::channel();

        let ctrlc = async move {
            CtrlC::new().expect("cannot create Ctrl+C handler").await;
            info!("received interruption signal, exiting envelopes watcher…");
            request_shutdown.send(()).unwrap();
            wait_for_shutdown.await.unwrap();
            Ok(())
        };

        let r#loop = async {
            let res = self
                .watch_envelopes_loop(folder, &mut wait_for_shutdown_request)
                .await;
            shutdown.send(()).unwrap();
            res
        };

        tokio::select! {
            res = ctrlc => res,
            res = r#loop => res,
        }
    }
}
