use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{imap::ImapContextSync, Result};

use super::{Folders, ListFolders};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot list imap folders")]
    ListFoldersError(#[source] imap::Error),
}

#[derive(Debug)]
pub struct ListImapFolders {
    ctx: ImapContextSync,
}

impl ListImapFolders {
    pub fn new(ctx: impl Into<ImapContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<ImapContextSync>) -> Box<dyn ListFolders> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl ListFolders for ListImapFolders {
    async fn list_folders(&self) -> Result<Folders> {
        info!("listing imap folders");

        let config = &self.ctx.account_config;
        let mut ctx = self.ctx.lock().await;

        let names = ctx
            .exec(
                |session| session.list(Some(""), Some("*")),
                |err| Error::ListFoldersError(err).into(),
            )
            .await?;

        let folders = Folders::from_imap_names(config, names);

        Ok(folders)
    }
}
