use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, flag::Flag, imap::ImapContextSync, Result};

use super::RemoveMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot add deleted flag to imap message(s) {2} from folder {1}")]
    AddDeletedFlagError(#[source] imap::Error, String, Id),
}

#[derive(Clone)]
pub struct RemoveImapMessages {
    ctx: ImapContextSync,
}

impl RemoveImapMessages {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn RemoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn RemoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveMessages for RemoveImapMessages {
    async fn remove_messages(&self, folder: &str, id: &Id) -> Result<()> {
        info!("removing imap messages {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded from folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.clone()).into(),
        )
        .await?;

        ctx.exec(
            |session| {
                let query = format!("+FLAGS ({})", Flag::Deleted.to_imap_query_string());
                session.uid_store(id.join(","), query)
            },
            |err| Error::AddDeletedFlagError(err, folder.clone(), id.clone()).into(),
        )
        .await?;

        Ok(())
    }
}
