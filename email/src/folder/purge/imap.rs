use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    email::{Flag, Flags},
    imap::ImapSessionManagerSync,
    Result,
};

use super::PurgeFolder;

#[derive(Debug)]
pub struct PurgeImapFolder {
    session_manager: ImapSessionManagerSync,
}

impl PurgeImapFolder {
    pub fn new(session_manager: ImapSessionManagerSync) -> Box<dyn PurgeFolder> {
        Box::new(Self { session_manager })
    }
}

#[async_trait]
impl PurgeFolder for PurgeImapFolder {
    async fn purge_folder(&self, folder: &str) -> Result<()> {
        info!("purging imap folder {folder}");

        let mut session = self.session_manager.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let flags = Flags::from_iter([Flag::Deleted]);
        let uids = String::from("1:*");

        session
            .execute(|session| {
                session.select(&folder_encoded)?;
                session.uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query_string()))?;
                session.expunge()
            })
            .await?;

        Ok(())
    }
}
