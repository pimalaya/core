use std::fs;

use async_trait::async_trait;

use super::DeleteFolder;
use crate::{
    folder::{error::Error, FolderKind},
    info,
    maildir::{self, MaildirContextSync},
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

        let path = if FolderKind::matches_inbox(folder) {
            ctx.root.path().join("cur")
        } else {
            let folder = maildir::encode_folder(folder);
            ctx.root.path().join(format!(".{}", folder))
        };

        fs::remove_dir_all(&path).map_err(|err| Error::DeleteFolderMaildirError(err, path))?;

        Ok(())
    }
}
