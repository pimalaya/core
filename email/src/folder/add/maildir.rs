use async_trait::async_trait;
use tracing::info;

use super::AddFolder;
use crate::{folder::error::Error, maildir::MaildirContextSync, AnyResult};

pub struct AddMaildirFolder {
    ctx: MaildirContextSync,
}

impl AddMaildirFolder {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn AddFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn AddFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddFolder for AddMaildirFolder {
    async fn add_folder(&self, folder: &str) -> AnyResult<()> {
        info!("creating maildir folder {folder}");

        let ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        ctx.root
            .create(config.get_folder_alias(folder))
            .map_err(|e| Error::CreateFolderStructureMaildirError(e, ctx.root.path().to_owned()))?;

        Ok(())
    }
}
