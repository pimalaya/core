use async_trait::async_trait;
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::DeleteFolder;
use crate::{imap::ImapContext, AnyResult};

#[derive(Debug)]
pub struct DeleteImapFolder {
    ctx: ImapContext,
}

impl DeleteImapFolder {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn DeleteFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn DeleteFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl DeleteFolder for DeleteImapFolder {
    async fn delete_folder(&self, folder: &str) -> AnyResult<()> {
        info!("deleting imap folder {folder}");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        client.delete_mailbox(&folder_encoded).await?;

        Ok(())
    }
}
