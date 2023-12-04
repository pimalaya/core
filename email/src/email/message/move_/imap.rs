use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, imap::ImapSessionSync, Result};

use super::MoveMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot move imap messages {3} from folder {1} to folder {2}")]
    MoveMessagesError(#[source] imap::Error, String, String, Id),
}

#[derive(Clone, Debug)]
pub struct MoveMessagesImap {
    session: ImapSessionSync,
}

impl MoveMessagesImap {
    pub fn new(session: &ImapSessionSync) -> Option<Box<dyn MoveMessages>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl MoveMessages for MoveMessagesImap {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        info!("moving imap messages {id} from folder {from_folder} to folder {to_folder}");

        let mut session = self.session.lock().await;

        let from_folder = session.account_config.get_folder_alias(from_folder)?;
        let from_folder_encoded = encode_utf7(from_folder.clone());
        debug!("utf7 encoded from folder: {from_folder_encoded}");

        let to_folder = session.account_config.get_folder_alias(to_folder)?;
        let to_folder_encoded = encode_utf7(to_folder.clone());
        debug!("utf7 encoded to folder: {to_folder_encoded}");

        session
            .execute(
                |session| session.select(&from_folder_encoded),
                |err| Error::SelectFolderError(err, from_folder.clone()).into(),
            )
            .await?;

        session
            .execute(
                |session| session.uid_mv(id.join(","), &to_folder_encoded),
                |err| {
                    Error::MoveMessagesError(
                        err,
                        from_folder.clone(),
                        to_folder.clone(),
                        id.clone(),
                    )
                    .into()
                },
            )
            .await?;

        Ok(())
    }
}
