use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapContextSync, Result};

use super::AddFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create imap folder {1}")]
    CreateFolderError(#[source] imap::Error, String),
}

#[derive(Clone, Debug)]
pub struct AddImapFolder {
    ctx: ImapContextSync,
}

impl AddImapFolder {
    pub fn new(ctx: impl Into<ImapContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<ImapContextSync>) -> Box<dyn AddFolder> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl AddFolder for AddImapFolder {
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("creating imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.create(&folder_encoded),
            |err| Error::CreateFolderError(err, folder.clone()).into(),
        )
        .await?;

        Ok(())
    }
}
