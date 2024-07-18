use async_trait::async_trait;

use super::DeleteFolder;
use crate::{
    folder::{error::Error, FolderKind},
    info,
    maildir::MaildirContextSync,
    AnyResult,
};

pub struct DeleteMaildirFolder {
    ctx: MaildirContextSync,
}

impl DeleteMaildirFolder {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn DeleteFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn DeleteFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl DeleteFolder for DeleteMaildirFolder {
    async fn delete_folder(&self, folder: &str) -> AnyResult<()> {
        info!("deleting maildir folder {folder}");

        let ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);

        if FolderKind::matches_inbox(&folder) {
            let path = ctx.root.path().to_owned();
            return Err(Error::DeleteMaildirInboxForbiddenError(path).into());
        }

        ctx.root
            .remove(&folder)
            .map_err(|err| Error::DeleteMaildirFolderError(err, folder))?;

        Ok(())
    }
}
