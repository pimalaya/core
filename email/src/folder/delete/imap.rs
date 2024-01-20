use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapContextSync, Result};

use super::DeleteFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot delete imap folder {1}")]
    DeleteFolderError(#[source] imap::Error, String),
}

#[derive(Debug)]
pub struct DeleteImapFolder {
    ctx: ImapContextSync,
}

impl DeleteImapFolder {
    pub fn new(ctx: impl Into<ImapContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<ImapContextSync>) -> Box<dyn DeleteFolder> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl DeleteFolder for DeleteImapFolder {
    async fn delete_folder(&self, folder: &str) -> Result<()> {
        info!("deleting imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.delete(&folder_encoded),
            |err| Error::DeleteFolderError(err, folder.clone()).into(),
        )
        .await?;

        Ok(())
    }
}
