use async_trait::async_trait;
use log::{debug, info};
use std::error;
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::envelope::MultipleIds, imap::ImapSessionSync, Result};

use super::{Messages, PeekMessages};

/// The IMAP query needed to retrieve messages.
const PEEK_MESSAGES_QUERY: &str = "BODY.PEEK[]";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot peek imap messages {2} from folder {1}")]
    PeekMessagesError(#[source] imap::Error, String, MultipleIds),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn peek_messages(
        err: imap::Error,
        folder: String,
        ids: MultipleIds,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::PeekMessagesError(err, folder, ids))
    }
}

#[derive(Clone, Debug)]
pub struct PeekImapMessages {
    session: ImapSessionSync,
}

impl PeekImapMessages {
    pub fn new(session: &ImapSessionSync) -> Box<dyn PeekMessages> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl PeekMessages for PeekImapMessages {
    async fn peek_messages(&self, folder: &str, ids: &MultipleIds) -> Result<Messages> {
        info!("peeking messages {ids} from folder {folder}");

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
                |session| session.uid_fetch(ids.join(","), PEEK_MESSAGES_QUERY),
                |err| Error::peek_messages(err, folder.clone(), ids.clone()),
            )
            .await?;

        Ok(Messages::try_from(fetches)?)
    }
}
