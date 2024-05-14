use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::PurgeFolder;
use crate::{debug, imap::ImapContextSync, info, AnyResult};

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
    async fn purge_folder(&self, folder: &str) -> AnyResult<()> {
        info!("purging imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.purge_mailbox(&folder_encoded).await?;

        Ok(())
    }
}
