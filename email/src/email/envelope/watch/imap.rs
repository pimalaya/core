use async_trait::async_trait;
use imap::extensions::idle::stop_on_any;
use log::{debug, info};
use std::{collections::HashMap, time::Duration};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    envelope::{list::imap::LIST_ENVELOPES_QUERY, Envelope, Envelopes},
    imap::ImapContextSync,
    Result,
};

use super::WatchEnvelopes;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot examine imap folder {1}")]
    ExamineFolderImapError(#[source] imap::Error, String),
    #[error("cannot run imap idle mode")]
    RunIdleModeImapError(#[source] imap::Error),
    #[error("cannot list all imap envelopes of folder {1}")]
    ListAllEnvelopesImapError(#[source] imap::Error, String),
}

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
}

#[async_trait]
impl WatchEnvelopes for WatchImapEnvelopes {
    async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        info!("watching imap folder {folder} for envelope changes");

        let config = &self.ctx.account_config;
        let timeout = &self.ctx.imap_config.find_watch_timeout();
        let mut ctx = self.ctx.lock().await;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let envelopes_count = ctx
            .exec(
                |session| session.examine(&folder_encoded),
                |err| Error::ExamineFolderImapError(err, folder.clone()).into(),
            )
            .await?
            .exists;

        let envelopes = if envelopes_count == 0 {
            Default::default()
        } else {
            let fetches = ctx
                .exec(
                    |session| session.fetch("1:*", LIST_ENVELOPES_QUERY),
                    |err| Error::ListAllEnvelopesImapError(err, folder.clone()).into(),
                )
                .await?;
            Envelopes::from_imap_fetches(fetches)
        };

        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        loop {
            debug!("starting idle loop…");

            ctx.exec(
                |session| {
                    let mut idle = session.idle();

                    if let Some(secs) = timeout {
                        debug!("setting imap idle timeout option at {secs}secs");
                        idle.timeout(Duration::new(*secs, 0));
                    }

                    idle.wait_while(stop_on_any)
                },
                |err| Error::RunIdleModeImapError(err).into(),
            )
            .await?;

            debug!("exitting the idle loop");

            let fetches = ctx
                .exec(
                    |session| session.fetch("1:*", LIST_ENVELOPES_QUERY),
                    |err| Error::ListAllEnvelopesImapError(err, folder.clone()).into(),
                )
                .await?;
            let next_envelopes = Envelopes::from_imap_fetches(fetches);
            let next_envelopes: HashMap<String, Envelope> =
                HashMap::from_iter(next_envelopes.into_iter().map(|e| (e.id.clone(), e)));

            self.exec_hooks(config, &envelopes, &next_envelopes).await;

            envelopes = next_envelopes;
        }
    }
}
