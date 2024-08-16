use async_trait::async_trait;
use imap_next::imap_types::sequence::{Sequence, SequenceSet};
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::{Flags, RemoveFlags};
use crate::{debug, envelope::Id, imap::ImapContextSync, info, AnyResult, Error};

#[derive(Clone, Debug)]
pub struct RemoveImapFlags {
    ctx: ImapContextSync,
}

impl RemoveImapFlags {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn RemoveFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn RemoveFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveFlags for RemoveImapFlags {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> AnyResult<()> {
        info!("removing imap flag(s) {flags} to envelope {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let uids: SequenceSet = match id {
            Id::Single(id) => Sequence::try_from(id.as_str())
                .map_err(Error::ParseSequenceError)?
                .into(),
            Id::Multiple(ids) => ids
                .iter()
                .filter_map(|id| {
                    let seq = Sequence::try_from(id.as_str());

                    #[cfg(feature = "tracing")]
                    if let Err(err) = &seq {
                        tracing::debug!(?id, ?err, "skipping invalid sequence");
                    }

                    seq.ok()
                })
                .collect::<Vec<_>>()
                .try_into()
                .map_err(Error::ParseSequenceError)?,
        };

        ctx.select_mailbox(&folder_encoded).await?;
        ctx.remove_flags(uids, flags.to_imap_flags_iter()).await?;

        Ok(())
    }
}
