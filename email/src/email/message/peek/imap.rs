use async_trait::async_trait;
use imap_client::imap_next::imap_types::sequence::{Sequence, SequenceSet};
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{Messages, PeekMessages};
use crate::{envelope::Id, imap::ImapContext, AnyResult};

#[derive(Clone, Debug)]
pub struct PeekImapMessages {
    ctx: ImapContext,
}

impl PeekImapMessages {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn PeekMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn PeekMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PeekMessages for PeekImapMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        info!("peeking imap messages {id} from folder {folder}");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let uids: SequenceSet = match id {
            Id::Single(id) => Sequence::try_from(id.as_str()).unwrap().into(),
            Id::Multiple(ids) => ids
                .iter()
                .filter_map(|id| Sequence::try_from(id.as_str()).ok())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        };

        client.select_mailbox(&folder_encoded).await?;
        let msgs = client.peek_messages(uids).await?;

        Ok(msgs)
    }
}
