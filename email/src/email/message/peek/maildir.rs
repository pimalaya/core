use async_trait::async_trait;
use log::{info, warn};

use crate::{envelope::Id, maildir::MaildirSessionSync, Result};

use super::{Messages, PeekMessages};

#[derive(Clone)]
pub struct PeekMessagesMaildir {
    session: MaildirSessionSync,
}

impl PeekMessagesMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn PeekMessages>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl PeekMessages for PeekMessagesMaildir {
    async fn peek_messages(&self, folder: &str, id: &Id) -> Result<Messages> {
        info!("peeking messages {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        let mut msgs: Vec<(usize, maildirpp::MailEntry)> = mdir
            .list_cur()
            .filter_map(|entry| match entry {
                Ok(entry) => id
                    .iter()
                    .position(|id| id == entry.id())
                    .map(|pos| (pos, entry)),
                Err(err) => {
                    warn!("skipping invalid maildir entry: {}", err);
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
