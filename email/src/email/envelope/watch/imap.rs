use async_trait::async_trait;
use imap::extensions::idle::stop_on_any;
use log::{debug, info};
use std::{collections::HashMap, time::Duration};
use thiserror::Error;
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
    #[error("cannot start imap idle mode")]
    StartIdleModeError(#[source] imap::Error),
    #[error("cannot list all envelopes of folder {1}")]
    ListAllEnvelopesError(#[source] imap::Error, String),
}

#[derive(Clone, Debug)]
pub struct WatchImapEmails {
    ctx: ImapSessionSync,
}

impl WatchImapEmails {
    pub fn new(ctx: &ImapSessionSync) -> Option<Box<dyn WatchEnvelopes>> {
        let ctx = ctx.clone();
        Some(Box::new(Self { ctx }))
    }
}

#[async_trait]
impl WatchEnvelopes for WatchImapEmails {
    async fn watch_envelopes(&self, folder: &str) -> Result<()> {
        info!("imap: watching folder {folder} for email changes");

        let mut ctx = self.ctx.lock().await;

        let folder = ctx.account_config.get_folder_alias(folder)?;
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

        loop {
            debug!("watching imap folder {folder:?}…");
            ctx.execute(
                |session| {
                    session
                        .idle()
                        .timeout(Duration::new(60, 0))
                        .wait_while(stop_on_any)
                },
                |err| Error::StartIdleModeError(err).into(),
            )
            .await?;

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
    }
}
