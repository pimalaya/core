use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionSync, Result};

use super::ExpungeFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderError(#[source] imap::Error, String),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn expunge_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::ExpungeFolderError(err, folder))
    }
}

#[derive(Debug)]
pub struct ExpungeFolderImap {
    session: ImapSessionSync,
}

impl ExpungeFolderImap {
    pub fn new(session: &ImapSessionSync) -> Box<dyn ExpungeFolder> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeFolderImap {
    async fn expunge_folder(&self, folder: &str) -> Result<()> {
        info!("expunging imap folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(
                |session| session.select(&folder_encoded),
                |err| Error::select_folder(err, folder.clone()),
            )
            .await?;

        session
            .execute(
                |session| session.expunge(),
                |err| Error::expunge_folder(err, folder.clone()),
            )
            .await?;

        Ok(())
    }
}
