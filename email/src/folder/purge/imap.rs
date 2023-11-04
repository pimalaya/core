use std::error;

use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    email::{Flag, Flags},
    imap::ImapSessionSync,
    Result,
};

use super::PurgeFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot add imap flag deleted to all envelopes in folder {1}")]
    AddDeletedFlagError(#[source] imap::Error, String),
    #[error("cannot expunge imap folder {1}")]
    ExpungeFolderError(#[source] imap::Error, String),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn add_deleted_flag(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::AddDeletedFlagError(err, folder))
    }

    pub fn expunge_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::ExpungeFolderError(err, folder))
    }
}

#[derive(Debug)]
pub struct PurgeImapFolder {
    session: ImapSessionSync,
}

impl PurgeImapFolder {
    pub fn new(session: &ImapSessionSync) -> Box<dyn PurgeFolder> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl PurgeFolder for PurgeImapFolder {
    async fn purge_folder(&self, folder: &str) -> Result<()> {
        info!("purging imap folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let flags = Flags::from_iter([Flag::Deleted]);
        let uids = String::from("1:*");

        session
            .execute(
                |session| session.select(&folder_encoded),
                |err| Error::select_folder(err, folder.clone()),
            )
            .await?;

        session
            .execute(
                |session| {
                    session.uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query_string()))
                },
                |err| Error::add_deleted_flag(err, folder.clone()),
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
