use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, imap::ImapContextSync, Result};

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
    FetchEnvolpesError(#[source] imap::Error, String, Id),
    #[error("cannot find imap envelope {1} from folder {0}")]
    GetFirstEnvelopeError(String, Id),
}

#[derive(Clone, Debug)]
pub struct GetImapEnvelope {
    ctx: ImapContextSync,
}

impl GetImapEnvelope {
    pub fn new(ctx: impl Into<ImapContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<ImapContextSync>) -> Box<dyn GetEnvelope> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl GetEnvelope for GetImapEnvelope {
    async fn get_envelope(&self, folder: &str, id: &Id) -> Result<Envelope> {
        info!("getting imap envelope {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.clone()).into(),
        )
        .await?;

        let fetches = ctx
            .exec(
                |session| session.uid_fetch(id.to_string(), ENVELOPE_QUERY),
                |err| Error::FetchEnvolpesError(err, folder.clone(), id.clone()).into(),
            )
            .await?;

        let fetch = fetches
            .get(0)
            .ok_or_else(|| Error::GetFirstEnvelopeError(folder.clone(), id.clone()))?;

        let envelope = Envelope::from_imap_fetch(fetch)?;
        debug!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }
}
