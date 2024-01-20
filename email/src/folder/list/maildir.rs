use async_trait::async_trait;
use log::info;

use crate::{
    folder::{Folder, FolderKind, Folders},
    maildir::MaildirContextSync,
    Result,
};

use super::ListFolders;

pub struct ListMaildirFolders {
    ctx: MaildirContextSync,
}

impl ListMaildirFolders {
    pub fn new(ctx: impl Into<MaildirContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<MaildirContextSync>) -> Box<dyn ListFolders> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl ListFolders for ListMaildirFolders {
    async fn list_folders(&self) -> Result<Folders> {
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
