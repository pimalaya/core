use async_trait::async_trait;
use log::info;
use std::{fs, io, path::PathBuf};
use thiserror::Error;

use crate::{
    folder::FolderKind,
    maildir::{self, MaildirContextSync},
    Result,
};

use super::DeleteFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot delete maildir folder {1}")]
    DeleteFolderError(#[source] io::Error, PathBuf),
}

pub struct DeleteFolderMaildir {
    ctx: MaildirContextSync,
}

impl DeleteFolderMaildir {
    pub fn new(ctx: impl Into<MaildirContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<MaildirContextSync>) -> Box<dyn DeleteFolder> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl DeleteFolder for DeleteFolderMaildir {
    async fn delete_folder(&self, folder: &str) -> Result<()> {
        info!("deleting maildir folder {folder}");

        let ctx = self.ctx.lock().await;

        let path = if FolderKind::matches_inbox(folder) {
            ctx.root.path().join("cur")
        } else {
            let folder = maildir::encode_folder(folder);
            ctx.root.path().join(format!(".{}", folder))
        };

        fs::remove_dir_all(&path).map_err(|err| Error::DeleteFolderError(err, path))?;

        Ok(())
    }
}
