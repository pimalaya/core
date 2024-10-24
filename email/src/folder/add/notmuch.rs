use async_trait::async_trait;
use tracing::info;

use super::AddFolder;
use crate::{folder::error::Error, notmuch::NotmuchContextSync, AnyResult};

pub struct AddNotmuchFolder {
    ctx: NotmuchContextSync,
}

impl AddNotmuchFolder {
    pub fn new(ctx: &NotmuchContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &NotmuchContextSync) -> Box<dyn AddFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &NotmuchContextSync) -> Option<Box<dyn AddFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddFolder for AddNotmuchFolder {
    async fn add_folder(&self, folder: &str) -> AnyResult<()> {
        info!("creating notmuch folder {folder} via maildir");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;

        ctx.mdir_ctx
            .root
            .create(config.get_folder_alias(folder))
            .map_err(|e| Error::CreateFolderStructureNotmuchError(e, folder.to_owned()))?;

        Ok(())
    }
}
