use async_trait::async_trait;
use log::info;

use crate::{
    folder::{Folder, FolderKind, Folders},
    maildir::MaildirContextSync,
};

use super::ListFolders;

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
    async fn list_folders(&self) -> crate::Result<Folders> {
        info!("listing maildir folders");

        let ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let mut folders = Folders::default();

        folders.push(Folder {
            kind: Some(FolderKind::Inbox),
            name: config.get_inbox_folder_alias(),
            desc: ctx.root.path().to_string_lossy().to_string(),
        });

        let subfolders: Vec<Folder> =
            Folders::from_submaildirs(config, ctx.root.list_subdirs()).into();

        folders.extend(subfolders);

        Ok(folders)
    }
}
