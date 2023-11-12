use async_trait::async_trait;
use log::info;
use std::error;
use thiserror::Error;

use crate::{
    email::{envelope::Id, Flags},
    maildir::MaildirSessionSync,
    Result,
};

use super::AddFlags;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot add flags {3} to envelope(s) {2} from folder {1}")]
    AddFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

impl Error {
    pub fn add_flags(
        err: maildirpp::Error,
        folder: &str,
        id: &str,
        flags: &Flags,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::AddFlagsError(
            err,
            folder.to_owned(),
            id.to_owned(),
            flags.clone(),
        ))
    }
}

#[derive(Clone)]
pub struct AddFlagsMaildir {
    session: MaildirSessionSync,
}

impl AddFlagsMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn AddFlags> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl AddFlags for AddFlagsMaildir {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("maildir: adding flag(s) {flags} to envelope {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.add_flags(id, &flags.to_mdir_string())
                .map_err(|err| Error::add_flags(err, folder, id, flags))
        })?;

        Ok(())
    }
}
