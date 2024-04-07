use async_trait::async_trait;
use log::info;

use crate::{
    folder::{Folder, FolderKind, Folders},
    notmuch::NotmuchContextSync,
    AnyResult,
};

use super::ListFolders;

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
        let config = &ctx.account_config;
        let mdir_ctx = &ctx.mdir_ctx;

        let mut folders = Folders::default();

        folders.push(Folder {
            kind: Some(FolderKind::Inbox),
            name: config.get_inbox_folder_alias(),
            desc: mdir_ctx.root.path().to_string_lossy().to_string(),
        });

        let subfolders: Vec<Folder> =
            Folders::from_submaildirs(config, mdir_ctx.root.list_subdirs()).into();

        folders.extend(subfolders);

        Ok(folders)
    }
}
