use async_trait::async_trait;
use tracing::info;

use super::{Folders, ListFolders};
use crate::{imap::ImapContext, AnyResult};

#[derive(Debug, Clone)]
pub struct ListImapFolders {
    ctx: ImapContext,
}

impl ListImapFolders {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn ListFolders> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn ListFolders>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListFolders for ListImapFolders {
    async fn list_folders(&self) -> AnyResult<Folders> {
        info!("listing imap folders");

        let config = &self.ctx.account_config;
        let mut client = self.ctx.client().await;

        let folders = client.list_all_mailboxes(config).await?;

        Ok(folders)
    }
}
