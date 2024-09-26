use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{Envelope, GetEnvelope};
use crate::{debug, envelope::SingleId, imap::ImapContextSync, info, AnyResult};

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
    async fn get_envelope(&self, folder: &str, id: &SingleId) -> AnyResult<Envelope> {
        info!("getting imap envelope {id:?} from folder {folder}");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        client.select_mailbox(&folder_encoded).await?;

        let envelope = client.fetch_first_envelope(id.parse().unwrap()).await?;
        debug!("imap envelope: {envelope:#?}");

        Ok(envelope)
    }
}
