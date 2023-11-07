use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::envelope::Id, imap::ImapSessionSync, Result};

use super::CopyMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot copy imap messages {3} from folder {1} to folder {2}")]
    CopyMessagesError(#[source] imap::Error, String, String, Id),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn copy_messages(
        err: imap::Error,
        from_folder: String,
        to_folder: String,
        id: Id,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::CopyMessagesError(err, from_folder, to_folder, id))
    }
}

#[derive(Clone, Debug)]
pub struct CopyImapMessages {
    session: ImapSessionSync,
}

impl CopyImapMessages {
    pub fn new(session: &ImapSessionSync) -> Box<dyn CopyMessages> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl CopyMessages for CopyImapMessages {
    async fn copy_emails(&self, from_folder: &str, to_folder: &str, id: Id) -> Result<()> {
        info!("copying imap messages {id} from folder {from_folder} to folder {to_folder}");

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
                |err| Error::select_folder(err, from_folder.clone()),
            )
            .await?;

        session
            .execute(
                |session| session.uid_copy(id.join(","), &to_folder_encoded),
                |err| Error::copy_messages(err, from_folder.clone(), to_folder.clone(), id.clone()),
            )
            .await?;

        Ok(())
    }
}
