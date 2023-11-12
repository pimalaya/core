use async_trait::async_trait;
use log::{debug, info};
use std::{
    error,
    ffi::OsStr,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{account::DEFAULT_INBOX_FOLDER, maildir::MaildirSessionSync, Result};

use super::{Folder, Folders, ListFolders};

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot get subfolder from {1}")]
    GetSubfolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot parse subfolder {1} from {0}")]
    ParseSubfolderError(PathBuf, PathBuf),
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
}

pub struct ListFoldersMaildir {
    session: MaildirSessionSync,
}

impl ListFoldersMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn ListFolders> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl ListFolders for ListFoldersMaildir {
    async fn list_folders(&self) -> Result<Folders> {
        info!("listing maildir folders");

        let session = self.session.lock().await;

        let mut folders = Folders::default();

        folders.push(Folder {
            name: self.session.account_config.inbox_folder_alias()?,
            desc: DEFAULT_INBOX_FOLDER.into(),
        });

        for entry in session.list_subdirs() {
            let dir = entry.map_err(|err| Error::get_subfolder(err, session.path()))?;
            let dirname = dir.path().file_name();
            let name = dirname
                .and_then(OsStr::to_str)
                .and_then(|s| if s.len() < 2 { None } else { Some(&s[1..]) })
                .ok_or_else(|| Error::parse_subfolder(session.path(), dir.path()))?
                .to_string();

            if name == "notmuch" {
                continue;
            }

            folders.push(Folder {
                name: session.decode_folder(&name),
                desc: name,
            });
        }

        debug!("maildir folders: {:#?}", folders);

        Ok(folders)
    }
}
