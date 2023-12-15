use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirSessionSync, Result};

use super::{Flags, RemoveFlags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot remove flags {3} to envelope(s) {2} from folder {1}")]
    RemoveFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

#[derive(Clone)]
pub struct RemoveFlagsMaildir {
    session: MaildirSessionSync,
}

impl RemoveFlagsMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn RemoveFlags>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl RemoveFlags for RemoveFlagsMaildir {
    async fn remove_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("maildir: removing flag(s) {flags} to envelope {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_maildir_from_folder_name(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.remove_flags(id, &flags.to_mdir_string())
                .map_err(|err| {
                    Error::RemoveFlagsError(err, folder.to_owned(), id.to_string(), flags.clone())
                })
        })?;

        Ok(())
    }
}
