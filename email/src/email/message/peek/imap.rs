use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::envelope::Id, imap::ImapSessionSync, Result};

use super::{Messages, PeekMessages};

/// The IMAP query needed to retrieve messages.
const PEEK_MESSAGES_QUERY: &str = "BODY.PEEK[]";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot peek imap messages {2} from folder {1}")]
    PeekMessagesError(#[source] imap::Error, String, Id),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn peek_messages(err: imap::Error, folder: String, id: Id) -> Box<dyn error::Error + Send> {
        Box::new(Self::PeekMessagesError(err, folder, id))
    }
}

#[derive(Clone, Debug)]
pub struct PeekMessagesImap {
    session: ImapSessionSync,
}

impl PeekMessagesImap {
    pub fn new(session: &ImapSessionSync) -> Box<dyn PeekMessages> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl PeekMessages for PeekMessagesImap {
    async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        info!("peeking messages {id} from folder {folder}");

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
                |session| session.uid_fetch(id.join(","), PEEK_MESSAGES_QUERY),
                |err| Error::peek_messages(err, folder.clone(), id.clone()),
            )
            .await?;

        Ok(Messages::try_from(fetches)?)
    }
}
