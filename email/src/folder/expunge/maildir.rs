use async_trait::async_trait;
use log::info;
use std::path::PathBuf;
use thiserror::Error;

use crate::{maildir::MaildirContextSync, Result};

use super::ExpungeFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("maildir: cannot list current folder from {1}")]
    ListCurrentFolderError(#[source] maildirpp::Error, PathBuf),
    #[error("maildir: cannot delete message {2} from folder {1}")]
    DeleteMessageError(#[source] maildirpp::Error, PathBuf, String),
}

pub struct ExpungeMaildirFolder {
    ctx: MaildirContextSync,
}

impl ExpungeMaildirFolder {
    pub fn new(ctx: impl Into<MaildirContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<MaildirContextSync>) -> Box<dyn ExpungeFolder> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeMaildirFolder {
    async fn expunge_folder(&self, folder: &str) -> Result<()> {
        info!("expunging maildir folder {folder}");

        let ctx = self.ctx.lock().await;

        let mdir = ctx.get_maildir_from_folder_name(folder)?;
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
