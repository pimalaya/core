use async_trait::async_trait;
use tracing::info;

use super::ListFolders;
use crate::{folder::Folders, notmuch::NotmuchContextSync, AnyResult};

pub struct ListNotmuchFolders {
    ctx: NotmuchContextSync,
}

impl ListNotmuchFolders {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn ListFolders> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn ListFolders>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ListFolders for ListNotmuchFolders {
    async fn list_folders(&self) -> AnyResult<Folders> {
        info!("listing notmuch folders via maildir");

        let ctx = self.ctx.lock().await;
        let folders = Folders::from_maildir_context(&ctx.mdir_ctx);

        Ok(folders)
    }
}
