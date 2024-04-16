use crate::{debug, info};
use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::error::Error, envelope::Id, flag::Flag, imap::ImapContextSync, AnyResult};

use super::RemoveMessages;

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
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        info!("removing imap messages {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded from folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()),
        )
        .await?;

        ctx.exec(
            |session| {
                let query = format!("+FLAGS ({})", Flag::Deleted.to_imap_query_string());
                session.uid_store(id.join(","), query)
            },
            |err| Error::AddDeletedFlagImapError(err, folder.clone(), id.clone()),
        )
        .await?;

        Ok(())
    }
}
