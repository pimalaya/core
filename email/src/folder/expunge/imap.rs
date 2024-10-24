use async_trait::async_trait;
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::ExpungeFolder;
use crate::{imap::ImapContext, AnyResult};

#[derive(Debug)]
pub struct ExpungeImapFolder {
    ctx: ImapContext,
}

impl ExpungeImapFolder {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn ExpungeFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn ExpungeFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeImapFolder {
    async fn expunge_folder(&self, folder: &str) -> AnyResult<()> {
        info!("expunging imap folder {folder}");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let _count = client.expunge_mailbox(&folder_encoded).await?;
        debug!("expunged {_count} messages from {folder}");

        Ok(())
    }
}
