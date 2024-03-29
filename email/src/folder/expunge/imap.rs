use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapContextSync, Result};

use super::ExpungeFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderImapError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderImapError(#[source] imap::Error, String),
}

#[derive(Debug)]
pub struct ExpungeImapFolder {
    ctx: ImapContextSync,
}

impl ExpungeImapFolder {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn ExpungeFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ExpungeFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeImapFolder {
    async fn expunge_folder(&self, folder: &str) -> Result<()> {
        info!("expunging imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()).into(),
        )
        .await?;

        ctx.exec(
            |session| session.expunge(),
            |err| Error::ExpungeFolderImapError(err, folder.clone()).into(),
        )
        .await?;

        Ok(())
    }
}
