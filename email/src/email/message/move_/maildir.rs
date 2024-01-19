use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirSessionSync, Result};

use super::MoveMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot move messages {3} from maildir folder {1} to folder {2}")]
    MoveMessagesError(#[source] maildirpp::Error, String, String, String),
}

#[derive(Clone)]
pub struct MoveMessagesMaildir {
    session: MaildirSessionSync,
}

impl MoveMessagesMaildir {
    pub fn new(session: MaildirSessionSync) -> Self {
        Self { session }
    }

    pub fn new_boxed(session: MaildirSessionSync) -> Box<dyn MoveMessages> {
        Box::new(Self::new(session))
    }
}

#[async_trait]
impl MoveMessages for MoveMessagesMaildir {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        info!("moving maildir messages {id} from folder {from_folder} to folder {to_folder}");

        let session = self.session.lock().await;
        let from_mdir = session.get_maildir_from_folder_name(from_folder)?;
        let to_mdir = session.get_maildir_from_folder_name(to_folder)?;

        id.iter().try_for_each(|id| {
            from_mdir.move_to(id, &to_mdir).map_err(|err| {
                Error::MoveMessagesError(
                    err,
                    from_folder.to_owned(),
                    to_folder.to_owned(),
                    id.to_owned(),
                )
            })
        })?;

        Ok(())
    }
}
