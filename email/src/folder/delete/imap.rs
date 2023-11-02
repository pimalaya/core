use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionManagerSync, Result};

use super::DeleteFolder;

#[derive(Debug)]
pub struct DeleteImapFolder {
    session_manager: ImapSessionManagerSync,
}

impl DeleteImapFolder {
    pub fn new(session_manager: ImapSessionManagerSync) -> Box<dyn DeleteFolder> {
        Box::new(Self { session_manager })
    }
}

#[async_trait]
impl DeleteFolder for DeleteImapFolder {
    async fn delete_folder(&self, folder: &str) -> Result<()> {
        info!("deleting imap folder {folder}");

        let mut session = self.session_manager.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(|session| session.delete(&folder_encoded))
            .await?;

        Ok(())
    }
}
