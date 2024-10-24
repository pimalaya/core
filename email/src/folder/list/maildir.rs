use async_trait::async_trait;
use tracing::info;

use super::ListFolders;
use crate::{folder::Folders, maildir::MaildirContextSync, AnyResult};

pub struct ListMaildirFolders {
    ctx: MaildirContextSync,
}

impl ListMaildirFolders {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn ListFolders> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ListFolders>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListFolders for ListMaildirFolders {
    async fn list_folders(&self) -> AnyResult<Folders> {
        info!("listing maildir folders");

        let ctx = self.ctx.lock().await;
        let folders = Folders::from_maildir_context(&ctx);

        Ok(folders.into())
    }
}
