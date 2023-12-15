use async_trait::async_trait;
use log::info;
use maildirpp::Maildir;
use std::path::PathBuf;
use thiserror::Error;

use crate::{
    folder::FolderKind,
    maildir::{self, MaildirSessionSync},
    Result,
};

use super::AddFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderError(PathBuf, PathBuf),
    #[error("cannot create maildir {1} folder structure")]
    InitFolderError(#[source] maildirpp::Error, PathBuf),
}

pub struct AddFolderMaildir {
    session: MaildirSessionSync,
}

impl AddFolderMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn AddFolder>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl AddFolder for AddFolderMaildir {
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("adding folder {folder}");

        let session = self.session.lock().await;

        let path = if FolderKind::matches_inbox(folder) {
            session.path().join("cur")
        } else {
            let folder = maildir::encode_folder(folder);
            session.path().join(format!(".{}", folder))
        };

        Maildir::from(path.clone())
            .create_dirs()
            .map_err(|err| Error::InitFolderError(err, path))?;

        Ok(())
    }
}
