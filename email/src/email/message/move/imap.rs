use async_trait::async_trait;
use imap_client::imap_next::imap_types::sequence::{Sequence, SequenceSet};
use tracing::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::MoveMessages;
use crate::{envelope::Id, imap::ImapContext, AnyResult};

#[derive(Clone, Debug)]
pub struct MoveImapMessages {
    pub(crate) ctx: ImapContext,
}

impl MoveImapMessages {
    pub fn new(ctx: &ImapContext) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContext) -> Box<dyn MoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContext) -> Option<Box<dyn MoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl MoveMessages for MoveImapMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> AnyResult<()> {
        info!("moving imap messages {id} from folder {from_folder} to folder {to_folder}");

        let mut client = self.ctx.client().await;
        let config = &client.account_config;

        let from_folder = config.get_folder_alias(from_folder);
        let from_folder_encoded = encode_utf7(from_folder.clone());
        debug!("utf7 encoded from folder: {from_folder_encoded}");

        let to_folder = config.get_folder_alias(to_folder);
        let to_folder_encoded = encode_utf7(to_folder.clone());
        debug!("utf7 encoded to folder: {to_folder_encoded}");

        let uids: SequenceSet = match id {
            Id::Single(id) => Sequence::try_from(id.as_str()).unwrap().into(),
            Id::Multiple(ids) => ids
                .iter()
                .filter_map(|id| Sequence::try_from(id.as_str()).ok())
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        };

        client.select_mailbox(&from_folder_encoded).await?;
        client.move_messages(uids, &to_folder_encoded).await?;

        Ok(())
    }
}
