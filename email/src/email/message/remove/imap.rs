use async_trait::async_trait;
use imap_client::imap_next::imap_types::sequence::{Sequence, SequenceSet};
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::RemoveMessages;
use crate::{envelope::Id, imap::ImapContext, AnyResult};

#[derive(Clone)]
pub struct RemoveImapMessages {
    ctx: ImapContext,
}

impl RemoveImapMessages {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn RemoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn RemoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveMessages for RemoveImapMessages {
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        info!("removing imap messages {id} from folder {folder}");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded from folder: {folder_encoded}");

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
        client.add_deleted_flag(uids).await?;

        Ok(())
    }
}
