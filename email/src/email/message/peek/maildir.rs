use async_trait::async_trait;
use tracing::info;

use super::{Messages, PeekMessages};
use crate::{envelope::Id, maildir::MaildirContextSync, AnyResult, Error};

#[derive(Clone)]
pub struct PeekMaildirMessages {
    ctx: MaildirContextSync,
}

impl PeekMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn PeekMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn PeekMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PeekMessages for PeekMaildirMessages {
    async fn peek_messages(&self, folder: &str, id: &Id) -> AnyResult<Messages> {
        info!("peeking maildir messages {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let mut msgs: Vec<(usize, maildirs::MaildirEntry)> = mdir
            .read()
            .map_err(Error::ListMaildirEntriesError)?
            .filter_map(|entry| {
                let mut entry = (entry, String::new());
                match entry.0.id() {
                    Err(_) => None,
                    Ok(id) => {
                        entry.1 = id.to_owned();
                        Some(entry)
                    }
                }
            })
            .filter_map(|(entry, entry_id)| {
                id.iter()
                    .position(|id| id == entry_id)
                    .map(|pos| (pos, entry))
            })
            .collect();
        msgs.sort_by_key(|(pos, _)| *pos);

        let msgs: Messages = msgs
            .into_iter()
            .map(|(_, entry)| entry)
            .collect::<Vec<_>>()
            .try_into()?;

        Ok(msgs)
    }
}
