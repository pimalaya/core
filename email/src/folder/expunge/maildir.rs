use async_trait::async_trait;
use log::info;
use std::path::PathBuf;
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

pub struct ExpungeFolderMaildir {
    session: MaildirSessionSync,
}

impl ExpungeFolderMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn ExpungeFolder>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
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
            .map_err(|err| Error::ListCurrentFolderError(err, mdir.path().to_owned()))?;
        entries
            .iter()
            .filter_map(|entry| {
                if entry.is_trashed() {
                    Some(entry.id())
                } else {
                    None
                }
            })
            .try_for_each(|id| {
                mdir.delete(id).map_err(|err| {
                    Error::DeleteMessageError(err, mdir.path().to_owned(), id.to_owned())
                })
            })?;

        Ok(())
    }
}
