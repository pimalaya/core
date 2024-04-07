use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    email::error::Error,
    envelope::{list::imap::LIST_ENVELOPES_QUERY, Id},
    imap::ImapContextSync,
    AnyResult,
};

use super::{Envelope, GetEnvelope};

#[derive(Clone, Debug)]
pub struct GetImapEnvelope {
    ctx: ImapContextSync,
}

impl GetImapEnvelope {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn GetEnvelope> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn GetEnvelope>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl GetEnvelope for GetImapEnvelope {
    async fn get_envelope(&self, folder: &str, id: &Id) -> AnyResult<Envelope> {
        info!("getting imap envelope {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()),
        )
        .await?;

        let fetches = ctx
            .exec(
                |session| session.uid_fetch(id.to_string(), LIST_ENVELOPES_QUERY),
                |err| Error::FetchEnvolpesImapError(err, folder.clone(), id.clone()),
            )
            .await?;

        let fetch = fetches
            .get(0)
            .ok_or_else(|| Error::GetFirstEnvelopeImapError(folder.clone(), id.clone()))?;

        let envelope = Envelope::from_imap_fetch(fetch)?;
        debug!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }
}
