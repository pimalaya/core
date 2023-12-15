use async_trait::async_trait;
use log::info;
use std::{fs, io, path::PathBuf};
use thiserror::Error;

use crate::{
    folder::FolderKind,
    maildir::{self, MaildirSessionSync},
    Result,
};

use super::DeleteFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot delete folder {1}")]
    DeleteFolderError(#[source] io::Error, PathBuf),
}

pub struct DeleteFolderMaildir {
    session: MaildirSessionSync,
}

impl DeleteFolderMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn DeleteFolder>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl DeleteFolder for DeleteFolderMaildir {
    async fn delete_folder(&self, folder: &str) -> Result<()> {
        info!("deleting folder {folder}");

        let session = self.session.lock().await;

        let path = if FolderKind::matches_inbox(folder) {
            session.path().join("cur")
        } else {
            let folder = maildir::encode_folder(folder);
            session.path().join(format!(".{}", folder))
        };

        fs::remove_dir_all(&path).map_err(|err| Error::DeleteFolderError(err, path))?;

        Ok(())
    }
}
