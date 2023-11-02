use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionManagerSync, Result};

use super::ExpungeFolder;

#[derive(Debug)]
pub struct ExpungeImapFolder {
    session_manager: ImapSessionManagerSync,
}

impl ExpungeImapFolder {
    pub fn new(session_manager: ImapSessionManagerSync) -> Box<dyn ExpungeFolder> {
        Box::new(Self { session_manager })
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeImapFolder {
    async fn expunge_folder(&self, folder: &str) -> Result<()> {
        info!("expunging imap folder {folder}");

        let mut session = self.session_manager.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(|session| {
                session.select(&folder_encoded)?;
                session.expunge()
            })
            .await?;

        Ok(())
    }
}
