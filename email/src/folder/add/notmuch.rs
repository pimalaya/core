use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use std::path::PathBuf;
use thiserror::Error;

use crate::{folder::FolderKind, maildir, notmuch::NotmuchContextSync, Result};

use super::AddFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create notmuch folder structure at {1}")]
    CreateFolderStructureError(#[source] maildirpp::Error, PathBuf),
}

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
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("creating notmuch folder {folder}");

        let config = &self.ctx.account_config;
        let ctx = self.ctx.lock().await;
        let mdir_ctx = &ctx.mdir_ctx;

        let path = if FolderKind::matches_inbox(folder) {
            mdir_ctx.root.path().to_owned()
        } else {
            let folder = config.get_folder_alias(folder);
            let folder = maildir::encode_folder(folder);
            mdir_ctx.root.path().join(format!(".{}", folder))
        };

        Maildir::from(path.clone())
            .create_dirs()
            .map_err(|err| Error::CreateFolderStructureError(err, path))?;

        Ok(())
    }
}
