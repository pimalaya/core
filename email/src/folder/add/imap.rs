use async_trait::async_trait;
use log::{debug, info};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSession, Result};

use super::AddFolder;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot create imap folder {1}")]
    CreateFolderError(#[source] imap::Error, String),
}

#[derive(Clone, Debug)]
pub struct AddFolderImap {
    session: Arc<Mutex<ImapSession>>,
}

impl AddFolderImap {
    pub fn new(session: Arc<Mutex<ImapSession>>) -> Self {
        Self { session }
    }

    pub fn new_boxed(session: Arc<Mutex<ImapSession>>) -> Box<dyn AddFolder> {
        Box::new(Self::new(session))
    }
}

#[async_trait]
impl AddFolder for AddFolderImap {
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("creating imap folder {folder}");

        let mut session = self.session.lock().await;
        let config = &session.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(
                |session| session.create(&folder_encoded),
                |err| Error::CreateFolderError(err, folder.clone()).into(),
            )
            .await?;

        Ok(())
    }
}
