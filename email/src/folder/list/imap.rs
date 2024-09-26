use async_trait::async_trait;

use super::{Folders, ListFolders};
use crate::{imap::ImapContextSync, info, AnyResult};

#[derive(Debug, Clone)]
pub struct ListImapFolders {
    ctx: ImapContextSync,
}

impl ListImapFolders {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn ListFolders> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn ListFolders>> {
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
