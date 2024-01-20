use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, imap::ImapContextSync, Result};

use super::{Messages, PeekMessages};

/// The IMAP query needed to retrieve messages.
const PEEK_MESSAGES_QUERY: &str = "BODY.PEEK[]";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot peek imap messages {2} from folder {1}")]
    PeekMessagesError(#[source] imap::Error, String, Id),
}

#[derive(Clone, Debug)]
pub struct PeekImapMessages {
    ctx: ImapContextSync,
}

impl PeekImapMessages {
    pub fn new(ctx: impl Into<ImapContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<ImapContextSync>) -> Box<dyn PeekMessages> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl PeekMessages for PeekImapMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        info!("peeking imap messages {id} from folder {folder}");

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
                |session| session.uid_fetch(id.join(","), PEEK_MESSAGES_QUERY),
                |err| Error::PeekMessagesError(err, folder.clone(), id.clone()).into(),
            )
            .await?;

        Ok(Messages::try_from(fetches)?)
    }
}
