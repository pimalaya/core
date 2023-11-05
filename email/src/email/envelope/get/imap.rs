use std::error;

use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{imap::ImapSessionSync, Result};

use super::{Envelope, GetEnvelope};

/// The IMAP query needed to retrieve everything we need to build an
/// [envelope]: UID, flags and headers (Message-ID, From, To, Subject,
/// Date).
const ENVELOPE_QUERY: &str =
    "(UID FLAGS BODY.PEEK[HEADER.FIELDS (MESSAGE-ID FROM TO SUBJECT DATE)])";

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot fetch imap envelopes {2} from folder {1}")]
    FetchEnvolpesError(#[source] imap::Error, String, String),
    #[error("cannot find envelope {1} from folder {0}")]
    GetFirstEnvelopeError(String, String),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn fetch_envelopes(
        err: imap::Error,
        folder: String,
        id: String,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::FetchEnvolpesError(err, folder, id))
    }

    pub fn get_first_envelope(folder: String, id: String) -> crate::imap::Error {
        crate::imap::Error::ExecuteSessionActionError(Box::new(Self::GetFirstEnvelopeError(
            folder, id,
        )))
    }
}

#[derive(Clone, Debug)]
pub struct GetImapEnvelope {
    session: ImapSessionSync,
}

impl GetImapEnvelope {
    pub fn new(session: &ImapSessionSync) -> Box<dyn GetEnvelope> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl GetEnvelope for GetImapEnvelope {
    async fn get_envelope(&mut self, folder: &str, id: &str) -> Result<Envelope> {
        info!("getting imap envelope {id} from folder {folder}");

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
                |session| session.uid_fetch(id, ENVELOPE_QUERY),
                |err| Error::fetch_envelopes(err, folder.clone(), id.to_owned()),
            )
            .await?;

        let fetch = fetches
            .get(0)
            .ok_or_else(|| Error::get_first_envelope(folder.clone(), id.to_owned()))?;

        let envelope = Envelope::from_imap_fetch(fetch)?;
        debug!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }
}
