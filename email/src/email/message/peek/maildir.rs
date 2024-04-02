use async_trait::async_trait;
use log::{debug, info};

use crate::{envelope::Id, maildir::MaildirContextSync};

use super::{Messages, PeekMessages};

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
    async fn peek_messages(&self, folder: &str, id: &Id) -> crate::Result<Messages> {
        info!("peeking maildir messages {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        let mut msgs: Vec<(usize, maildirpp::MailEntry)> = mdir
            .list_cur()
            .filter_map(|entry| match entry {
                Ok(entry) => id
                    .iter()
                    .position(|id| id == entry.id())
                    .map(|pos| (pos, entry)),
                Err(err) => {
                    debug!("skipping invalid maildir entry: {}", err);
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
