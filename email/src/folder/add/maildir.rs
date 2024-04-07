use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;

use crate::{
    folder::{error::Error, FolderKind},
    maildir::{self, MaildirContextSync},
    AnyResult,
};

use super::AddFolder;

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

        let path = if FolderKind::matches_inbox(folder) {
            ctx.root.path().to_owned()
        } else {
            let folder = config.get_folder_alias(folder);
            let folder = maildir::encode_folder(folder);
            ctx.root.path().join(format!(".{}", folder))
        };

        Maildir::from(path.clone())
            .create_dirs()
            .map_err(|e| Error::CreateFolderStructureMaildirError(e, path))?;

        Ok(())
    }
}
