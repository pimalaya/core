use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::ExpungeFolder;
use crate::{debug, folder::error::Error, imap::ImapContextSync, info, AnyResult};

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
    async fn expunge_folder(&self, folder: &str) -> AnyResult<()> {
        info!("expunging imap folder {folder}");

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

        ctx.exec(
            |session| session.expunge(),
            |err| Error::ExpungeFolderImapError(err, folder.clone()),
        )
        .await?;

        Ok(())
    }
}
