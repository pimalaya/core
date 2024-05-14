use async_trait::async_trait;
use imap_client::imap_flow::imap_codec::imap_types::sequence::{Sequence, SequenceSet};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{Envelope, GetEnvelope};
use crate::{debug, email::error::Error, envelope::Id, imap::ImapContextSync, info, AnyResult};

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

        ctx.select_mailbox(&folder_encoded).await?;

        let uids: SequenceSet = match id {
            Id::Single(id) => Sequence::try_from(id.as_str()).unwrap().into(),
            Id::Multiple(ids) => ids
                .iter()
                .filter_map(|id| Sequence::try_from(id.as_str()).ok())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        };

        let envelope = ctx
            .fetch_first_envelope(uids)
            .await?
            .ok_or_else(|| Error::GetFirstEnvelopeImapError(folder.clone(), id.clone()))?;
        debug!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }
}
