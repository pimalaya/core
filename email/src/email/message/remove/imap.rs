use async_trait::async_trait;
use imap_client::types::sequence::{Sequence, SequenceSet};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::RemoveMessages;
use crate::{debug, envelope::Id, imap::ImapContextSync, info, AnyResult};

#[derive(Clone)]
pub struct RemoveImapMessages {
    ctx: ImapContextSync,
}

impl RemoveImapMessages {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn RemoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn RemoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveMessages for RemoveImapMessages {
    async fn remove_messages(&self, folder: &str, id: &Id) -> AnyResult<()> {
        info!("removing imap messages {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

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

        ctx.select_mailbox(&folder_encoded).await?;
        ctx.add_deleted_flag(uids).await?;

        Ok(())
    }
}
