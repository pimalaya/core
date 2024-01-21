use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirContextSync, Result};

use super::{Flags, RemoveFlags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot remove flags {3} to envelope(s) {2} from folder {1}")]
    RemoveFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

#[derive(Clone)]
pub struct RemoveMaildirFlags {
    ctx: MaildirContextSync,
}

impl RemoveMaildirFlags {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn RemoveFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn RemoveFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl RemoveFlags for RemoveMaildirFlags {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("removing maildir flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.remove_flags(id, &flags.to_mdir_string())
                .map_err(|err| {
                    Error::RemoveFlagsError(err, folder.to_owned(), id.to_string(), flags.clone())
                })
        })?;

        Ok(())
    }
}
