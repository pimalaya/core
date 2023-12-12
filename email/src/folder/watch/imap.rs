use async_trait::async_trait;
use imap::extensions::idle::stop_on_any;
use log::{debug, info};
use std::time::Duration;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionSync, Result};

use super::WatchFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot examine imap folder {1}")]
    ExamineFolderError(#[source] imap::Error, String),
    #[error("cannot create imap folder {1}")]
    CreateFolderError(#[source] imap::Error, String),
    #[error("cannot start imap idle mode")]
    StartIdleModeError(#[source] imap::Error),
}

#[derive(Clone, Debug)]
pub struct WatchFolderImap {
    ctx: ImapSessionSync,
}

impl WatchFolderImap {
    pub fn new(ctx: &ImapSessionSync) -> Option<Box<dyn WatchFolder>> {
        let ctx = ctx.clone();
        Some(Box::new(Self { ctx }))
    }
}

#[async_trait]
impl WatchFolder for WatchFolderImap {
    async fn watch_folder(&self, folder: &str) -> Result<()> {
        info!("imap: watching folder {folder}");

        let mut ctx = self.ctx.lock().await;

        let folder = ctx.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.execute(
            |session| session.examine(&folder_encoded),
            |err| Error::ExamineFolderError(err, folder.clone()).into(),
        )
        .await?;

        loop {
            ctx.account_config.run_folder_watch_change_hooks().await;
            ctx.execute(
                |session| {
                    session
                        .idle()
                        .timeout(Duration::new(500, 0))
                        .wait_while(stop_on_any)
                },
                |err| Error::StartIdleModeError(err).into(),
            )
            .await?;
        }
    }
}
