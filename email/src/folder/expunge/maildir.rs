use async_trait::async_trait;
use log::info;
use std::{
    error,
    path::{Path, PathBuf},
};
use thiserror::Error;

use crate::{maildir::MaildirSessionSync, Result};

use super::ExpungeFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot list current folder from {1}")]
    ListCurrentFolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot delete message {2} from folder {1}")]
    DeleteMessageError(#[source] maildirpp::Error, PathBuf, String),
}

impl Error {
    pub fn list_current_folder(err: maildirpp::Error, path: &Path) -> Box<dyn error::Error + Send> {
        Box::new(Self::ListCurrentFolderError(err, path.to_owned()))
    }

    fn delete_message(
        err: maildirpp::Error,
        path: &Path,
        id: &str,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::DeleteMessageError(
            err,
            path.to_owned(),
            id.to_owned(),
        ))
    }
}

pub struct ExpungeFolderMaildir {
    session: MaildirSessionSync,
}

impl ExpungeFolderMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn ExpungeFolder> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeFolderMaildir {
    async fn expunge_folder(&self, folder: &str) -> Result<()> {
        info!("expunging folder {folder}");

        let session = self.session.lock().await;

        let mdir = session.get_mdir_from_dir(folder)?;
        let entries = mdir
            .list_cur()
            .collect::<maildirpp::Result<Vec<_>>>()
            .map_err(|err| Error::list_current_folder(err, mdir.path()))?;
        entries
            .iter()
            .filter_map(|entry| {
                if entry.is_trashed() {
                    Some(entry.id())
                } else {
                    None
                }
            })
            .try_for_each(|internal_id| {
                mdir.delete(internal_id)
                    .map_err(|err| Error::delete_message(err, mdir.path(), internal_id))
            })?;

        Ok(())
    }
}
