use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    flag::{Flag, Flags},
    imap::ImapContextSync,
    Result,
};

use super::PurgeFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderImapError(#[source] imap::Error, String),
    #[error("cannot add imap flag deleted to all envelopes in folder {1}")]
    AddDeletedFlagImapError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderImapError(#[source] imap::Error, String),
}

#[derive(Debug)]
pub struct PurgeImapFolder {
    ctx: ImapContextSync,
}

impl PurgeImapFolder {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn PurgeFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn PurgeFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PurgeFolder for PurgeImapFolder {
    async fn purge_folder(&self, folder: &str) -> Result<()> {
        info!("purging imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let flags = Flags::from_iter([Flag::Deleted]);
        let uids = String::from("1:*");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()).into(),
        )
        .await?;

        ctx.exec(
            |session| {
                session.uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query_string()))
            },
            |err| Error::AddDeletedFlagImapError(err, folder.clone()).into(),
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
