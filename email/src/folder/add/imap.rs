use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionManagerSafe, Result};

use super::AddFolder;

#[derive(Debug)]
pub struct AddImapFolder {
    session_manager: ImapSessionManagerSafe,
}

impl AddImapFolder {
    pub fn new(session_manager: ImapSessionManagerSafe) -> Box<dyn AddFolder> {
        Box::new(Self { session_manager })
    }
}

#[async_trait]
impl AddFolder for AddImapFolder {
    async fn add_folder(&self, folder: &str) -> Result<()> {
        info!("creating imap folder {folder}");

        let mut session_manager = self.session_manager.lock().await;

        let folder = session_manager.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session_manager
            .execute(|session| session.create(&folder_encoded))
            .await?;

        Ok(())
    }
}
