use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirContextSync, Result};

use super::CopyMessages;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot copy maildir messages {3} from folder {1} to folder {2}")]
    CopyMessagesError(#[source] maildirpp::Error, String, String, String),
}

#[derive(Clone)]
pub struct CopyMaildirMessages {
    ctx: MaildirContextSync,
}

impl CopyMaildirMessages {
    pub fn new(ctx: impl Into<MaildirContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<MaildirContextSync>) -> Box<dyn CopyMessages> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl CopyMessages for CopyMaildirMessages {
    async fn copy_messages(&self, from_folder: &str, to_folder: &str, id: &Id) -> Result<()> {
        info!("copying maildir messages {id} from folder {from_folder} to folder {to_folder}");

        let ctx = self.ctx.lock().await;
        let from_mdir = ctx.get_maildir_from_folder_name(from_folder)?;
        let to_mdir = ctx.get_maildir_from_folder_name(to_folder)?;

        id.iter().try_for_each(|id| {
            from_mdir.copy_to(id, &to_mdir).map_err(|err| {
                Error::CopyMessagesError(
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
