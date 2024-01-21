use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, imap::ImapContextSync, Result};

use super::{GetMessages, Messages};

/// The IMAP query needed to retrieve messages.
const GET_MESSAGES_QUERY: &str = "BODY[]";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot get imap messages {2} from folder {1}")]
    GetMessagesError(#[source] imap::Error, String, Id),
}

#[derive(Clone, Debug)]
pub struct GetImapMessages {
    ctx: ImapContextSync,
}

impl GetImapMessages {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn GetMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn GetMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl GetMessages for GetImapMessages {
    async fn get_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        info!("getting messages {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.clone()).into(),
        )
        .await?;

        let fetches = ctx
            .exec(
                |session| session.uid_fetch(id.join(","), GET_MESSAGES_QUERY),
                |err| Error::GetMessagesError(err, folder.clone(), id.clone()).into(),
            )
            .await?;

        Ok(Messages::try_from(fetches)?)
    }
}
