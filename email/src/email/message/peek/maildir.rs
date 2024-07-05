use async_trait::async_trait;

use super::{Messages, PeekMessages};
use crate::{debug, envelope::Id, info, maildir::MaildirContextSync, AnyResult};

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
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        let mut msgs: Vec<(usize, maildirs::MaildirEntry)> = mdir
            .list_cur()
            .filter_map(|entry| match entry {
                Ok(entry) => id
                    .iter()
                    .position(|id| id == entry.id())
                    .map(|pos| (pos, entry)),
                Err(_err) => {
                    debug!("skipping invalid maildir entry: {_err}");
                    None
                }
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
