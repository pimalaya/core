use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::error::Error, envelope::Id, imap::ImapContextSync, AnyResult};

use super::{Messages, PeekMessages};

/// The IMAP query needed to retrieve messages.
const PEEK_MESSAGES_QUERY: &str = "BODY.PEEK[]";

#[derive(Clone, Debug)]
pub struct PeekImapMessages {
    ctx: ImapContextSync,
}

impl PeekImapMessages {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn PeekMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn PeekMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PeekMessages for PeekImapMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        info!("peeking imap messages {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()),
        )
        .await?;

        let fetches = ctx
            .exec(
                |session| session.uid_fetch(id.join(","), PEEK_MESSAGES_QUERY),
                |err| Error::PeekMessagesImapError(err, folder.clone(), id.clone()),
            )
            .await?;

        Ok(Messages::try_from(fetches)?)
    }
}
