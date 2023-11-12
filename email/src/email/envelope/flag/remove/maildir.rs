use async_trait::async_trait;
use log::info;
use std::error;
use thiserror::Error;

use crate::{
    email::{envelope::Id, Flags},
    maildir::MaildirSessionSync,
    Result,
};

use super::RemoveFlags;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot remove flags {3} to envelope(s) {2} from folder {1}")]
    RemoveFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

impl Error {
    pub fn remove_flags(
        err: maildirpp::Error,
        folder: &str,
        id: &str,
        flags: &Flags,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::RemoveFlagsError(
            err,
            folder.to_owned(),
            id.to_owned(),
            flags.clone(),
        ))
    }
}

#[derive(Clone)]
pub struct RemoveFlagsMaildir {
    session: MaildirSessionSync,
}

impl RemoveFlagsMaildir {
    pub fn new(session: &MaildirSessionSync) -> Box<dyn RemoveFlags> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl RemoveFlags for RemoveFlagsMaildir {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("maildir: removing flag(s) {flags} to envelope {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.remove_flags(id, &flags.to_mdir_string())
                .map_err(|err| Error::remove_flags(err, folder, id, flags))
        })?;

        Ok(())
    }
}
