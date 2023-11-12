use async_trait::async_trait;
use log::{debug, info};
use maildirpp::Maildir;
use std::{
    error,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{account::DEFAULT_INBOX_FOLDER, maildir::MaildirSessionSync, Result};

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

impl Error {
    pub fn get_subfolder(err: maildirpp::Error, root_path: &Path) -> Box<dyn error::Error + Send> {
        Box::new(Self::GetSubfolderError(err, root_path.to_owned()))
    }

    pub fn parse_subfolder(root_path: &Path, path: &Path) -> Box<dyn error::Error + Send> {
        Box::new(Self::ParseSubfolderError(
            root_path.to_owned(),
            path.to_owned(),
        ))
    }

    pub fn init_folder(err: maildirpp::Error, path: &Path) -> Box<dyn error::Error + Send> {
        Box::new(Self::InitFolderError(err, path.to_owned()))
    }
}

pub struct AddFolderMaildir {
    session: MaildirSessionSync,
}

impl AddFolderMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn AddFolder> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl AddFolder for AddFolderMaildir {
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("adding folder {folder}");

        let session = self.session.lock().await;

        // FIXME: better check if given folder IS the inbox
        let path = match session.account_config.get_folder_alias(folder)?.as_str() {
            DEFAULT_INBOX_FOLDER => session.path().join("cur"),
            folder => {
                let folder = session.encode_folder(folder);
                session.path().join(format!(".{}", folder))
            }
        };

        debug!("folder path: {path:?}");

        Maildir::from(path.clone())
            .create_dirs()
            .map_err(|err| Error::init_folder(err, &path))?;

        Ok(())
    }
}
