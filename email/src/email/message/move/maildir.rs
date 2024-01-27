use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirContextSync, Result};

use super::MoveMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot move messages {3} from maildir folder {1} to folder {2}")]
    MoveMessagesError(#[source] maildirpp::Error, String, String, String),
}

#[derive(Clone)]
pub struct MoveMaildirMessages {
    pub(crate) ctx: MaildirContextSync,
}

impl MoveMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn MoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn MoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl MoveMessages for MoveMaildirMessages {
    async fn move_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        info!("moving maildir messages {id} from folder {from_folder} to folder {to_folder}");

        let ctx = self.ctx.lock().await;
        let from_mdir = ctx.get_maildir_from_folder_name(from_folder)?;
        let to_mdir = ctx.get_maildir_from_folder_name(to_folder)?;

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
