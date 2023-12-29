use async_trait::async_trait;
use futures::executor::block_on;
use log::{debug, info};
use std::{collections::HashMap, time::Duration};
use thiserror::Error;
use tokio::sync::mpsc;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    envelope::{list::imap::LIST_ENVELOPES_QUERY, Envelope, Envelopes},
    imap::ImapSessionSync,
    Result,
};

use super::WatchEnvelopes;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot examine imap folder {1}")]
    ExamineFolderError(#[source] imap::Error, String),
    #[error("cannot create imap folder {1}")]
    CreateFolderError(#[source] imap::Error, String),
    #[error("cannot run idle mode")]
    RunIdleModeError(#[source] imap::Error),
    #[error("cannot list all envelopes of folder {1}")]
    ListAllEnvelopesError(#[source] imap::Error, String),
}

#[derive(Clone, Debug)]
pub struct WatchImapEnvelopes {
    ctx: ImapSessionSync,
}

impl WatchImapEnvelopes {
    pub fn new(ctx: &ImapSessionSync) -> Option<Box<dyn WatchEnvelopes>> {
        let ctx = ctx.clone();
        Some(Box::new(Self { ctx }))
    }
}

#[async_trait]
impl WatchEnvelopes for WatchImapEnvelopes {
    async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        info!("imap: watching folder {folder} for email changes");

        let mut ctx = self.ctx.lock().await;

        let folder = ctx.account_config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.execute(
            |session| session.examine(&folder_encoded),
            |err| Error::ExamineFolderError(err, folder.clone()).into(),
        )
        .await?;

        let fetches = ctx
            .execute(
                |session| session.fetch("1:*", LIST_ENVELOPES_QUERY),
                |err| Error::ListAllEnvelopesError(err, folder.clone()).into(),
            )
            .await?;
        let envelopes = Envelopes::from_imap_fetches(fetches);
        let mut envelopes: HashMap<String, Envelope> =
            HashMap::from_iter(envelopes.into_iter().map(|e| (e.id.clone(), e)));

        let (tx, mut rx) = mpsc::channel(1);
        let timeout = ctx.imap_config.find_watch_timeout();

        debug!("watching imap folder {folder:?}…");
        ctx.execute(
            |session| {
                let mut idle = session.idle();

                if let Some(secs) = timeout {
                    debug!("setting imap idle timeout option at {secs}secs");
                    idle.timeout(Duration::new(secs, 0));
                }

                idle.wait_while(|res| {
                    if let Err(err) = block_on(tx.send(())) {
                        debug!("received imap error while idling: {res:?}");
                        debug!("{err:?}");
                    } else {
                        debug!("received unsolicited imap response while idling: {res:?}");
                    }

                    debug!("starting a new imap idle loop");
                    true
                })
            },
            |err| Error::RunIdleModeError(err).into(),
        )
        .await?;

        while let Some(()) = rx.recv().await {
            let fetches = ctx
                .execute(
                    |session| session.fetch("1:*", LIST_ENVELOPES_QUERY),
                    |err| Error::ListAllEnvelopesError(err, folder.clone()).into(),
                )
                .await?;
            let next_envelopes = Envelopes::from_imap_fetches(fetches);
            let next_envelopes: HashMap<String, Envelope> =
                HashMap::from_iter(next_envelopes.into_iter().map(|e| (e.id.clone(), e)));

            self.exec_hooks(&ctx.account_config, &envelopes, &next_envelopes)
                .await;

            envelopes = next_envelopes;
        }

        Ok(())
    }
}
