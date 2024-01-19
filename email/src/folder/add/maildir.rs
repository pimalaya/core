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
    #[error("cannot create maildir folder structure at {1}")]
    CreateFolderStructureError(#[source] maildirpp::Error, PathBuf),
}

pub struct AddMaildirFolder {
    session: MaildirSessionSync,
}

impl AddMaildirFolder {
    pub fn new(session: MaildirSessionSync) -> Self {
        Self { session }
    }

    pub fn new_boxed(session: MaildirSessionSync) -> Box<dyn AddFolder> {
        Box::new(Self::new(session))
    }
}

#[async_trait]
impl AddFolder for AddMaildirFolder {
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("adding maildir folder {folder}");

        let session = self.session.lock().await;
        let config = &session.account_config;

        let path = if FolderKind::matches_inbox(folder) {
            session.path().join("cur")
        } else {
            let folder = config.get_folder_alias(folder);
            let folder = maildir::encode_folder(folder);
            session.path().join(format!(".{}", folder))
        };

        Maildir::from(path.clone())
            .create_dirs()
            .map_err(|err| Error::CreateFolderStructureError(err, path))?;

        Ok(())
    }
}
