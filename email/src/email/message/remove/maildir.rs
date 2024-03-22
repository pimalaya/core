use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirContextSync, Result};

use super::RemoveMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot remove maildir message(s) {2} from folder {1}")]
    RemoveError(#[source] maildirpp::Error, String, String),
}

#[derive(Clone)]
pub struct RemoveMaildirMessages {
    ctx: MaildirContextSync,
}

impl RemoveMaildirMessages {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn RemoveMessages> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn RemoveMessages>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveMessages for RemoveMaildirMessages {
    async fn remove_messages(&self, folder: &str, id: &Id) -> Result<()> {
        info!("removing maildir message(s) {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.delete(id)
                .map_err(|err| Error::RemoveError(err, folder.to_owned(), id.to_string()))
        })?;

        Ok(())
    }
}
