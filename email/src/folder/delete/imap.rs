use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionSync, Result};

use super::DeleteFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot delete imap folder {1}")]
    DeleteFolderError(#[source] imap::Error, String),
}

impl Error {
    pub fn delete_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::DeleteFolderError(err, folder))
    }
}

#[derive(Debug)]
pub struct DeleteFolderImap {
    session: ImapSessionSync,
}

impl DeleteFolderImap {
    pub fn new(session: &ImapSessionSync) -> Box<dyn DeleteFolder> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl DeleteFolder for DeleteFolderImap {
    async fn delete_folder(&self, folder: &str) -> Result<()> {
        info!("deleting imap folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(
                |session| session.delete(&folder_encoded),
                |err| Error::delete_folder(err, folder.clone()),
            )
            .await?;

        Ok(())
    }
}
