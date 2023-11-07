use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::envelope::MultipleIds, imap::ImapSessionSync, Result};

use super::{GetMessages, Messages};

/// The IMAP query needed to retrieve messages.
const GET_MESSAGES_QUERY: &str = "BODY[]";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot get imap messages {2} from folder {1}")]
    GetMessagesError(#[source] imap::Error, String, MultipleIds),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn get_messages(
        err: imap::Error,
        folder: String,
        ids: MultipleIds,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::GetMessagesError(err, folder, ids))
    }
}

#[derive(Clone, Debug)]
pub struct GetImapMessages {
    session: ImapSessionSync,
}

impl GetImapMessages {
    pub fn new(session: &ImapSessionSync) -> Box<dyn GetMessages> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl GetMessages for GetImapMessages {
    async fn get_messages(&self, folder: &str, ids: &MultipleIds) -> Result<Messages> {
        info!("geting messages {ids} from folder {folder}");

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

        let fetches = session
            .execute(
                |session| session.uid_fetch(ids.join(","), GET_MESSAGES_QUERY),
                |err| Error::get_messages(err, folder.clone(), ids.clone()),
            )
            .await?;

        Ok(Messages::try_from(fetches)?)
    }
}
