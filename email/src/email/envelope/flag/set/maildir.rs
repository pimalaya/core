use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirContextSync, Result};

use super::{Flags, SetFlags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

#[derive(Clone)]
pub struct SetMaildirFlags {
    ctx: MaildirContextSync,
}

impl SetMaildirFlags {
    pub fn new(ctx: impl Into<MaildirContextSync>) -> Self {
        Self { ctx: ctx.into() }
    }

    pub fn new_boxed(ctx: impl Into<MaildirContextSync>) -> Box<dyn SetFlags> {
        Box::new(Self::new(ctx))
    }
}

#[async_trait]
impl SetFlags for SetMaildirFlags {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("setting maildir flag(s) {flags} to envelope {id} from folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_name(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.set_flags(id, &flags.to_mdir_string()).map_err(|err| {
                Error::SetFlagsError(err, folder.to_owned(), id.to_string(), flags.clone())
            })
        })?;

        Ok(())
    }
}
