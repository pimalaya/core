use async_trait::async_trait;
use log::info;
use std::error;
use thiserror::Error;

use crate::{
    email::{envelope::Id, Flags},
    maildir::MaildirSessionSync,
    Result,
};

use super::SetFlags;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

impl Error {
    pub fn set_flags(
        err: maildirpp::Error,
        folder: &str,
        id: &str,
        flags: &Flags,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::SetFlagsError(
            err,
            folder.to_owned(),
            id.to_owned(),
            flags.clone(),
        ))
    }
}

#[derive(Clone)]
pub struct SetFlagsMaildir {
    session: MaildirSessionSync,
}

impl SetFlagsMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn SetFlags> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl SetFlags for SetFlagsMaildir {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("maildir: setting flag(s) {flags} to envelope {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.set_flags(id, &flags.to_mdir_string())
                .map_err(|err| Error::set_flags(err, folder, id, flags))
        })?;

        Ok(())
    }
}
