use async_trait::async_trait;
use log::info;
use std::path::PathBuf;
use thiserror::Error;

use crate::{
    folder::{Folder, FolderKind, Folders},
    maildir::MaildirSessionSync,
    Result,
};

use super::ListFolders;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderError(PathBuf, PathBuf),
}

pub struct ListFoldersMaildir {
    session: MaildirSessionSync,
}

impl ListFoldersMaildir {
    pub fn new(session: MaildirSessionSync) -> Self {
        Self { session }
    }

    pub fn new_boxed(session: MaildirSessionSync) -> Box<dyn ListFolders> {
        Box::new(Self::new(session))
    }
}

#[async_trait]
impl ListFolders for ListFoldersMaildir {
    async fn list_folders(&self) -> Result<Folders> {
        info!("listing maildir folders");

        let session = self.session.lock().await;
        let config = &session.account_config;

        let mut folders = Folders::default();

        folders.push(Folder {
            kind: Some(FolderKind::Inbox),
            name: config.get_inbox_folder_alias(),
            desc: session.path().to_string_lossy().to_string(),
        });

        let subfolders: Vec<Folder> =
            Folders::from_submaildirs(config, session.list_subdirs()).into();
        folders.extend(subfolders);

        Ok(folders)
    }
}
