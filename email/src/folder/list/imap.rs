use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{imap::ImapSessionSync, Result};

use super::{Folders, ListFolders};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot list imap folders")]
    ListFoldersError(#[source] imap::Error),
}

#[derive(Debug)]
pub struct ListFoldersImap {
    session: ImapSessionSync,
}

impl ListFoldersImap {
    pub fn new(session: &ImapSessionSync) -> Option<Box<dyn ListFolders>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl ListFolders for ListFoldersImap {
    async fn list_folders(&self) -> Result<Folders> {
        info!("listing imap folders");

        let mut session = self.session.lock().await;

        let names = session
            .execute(
                |session| session.list(Some(""), Some("*")),
                |err| Error::ListFoldersError(err).into(),
            )
            .await?;

        let folders = Folders::from_imap_names(&session.account_config, names);

        Ok(folders)
    }
}
