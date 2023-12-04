use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::Id, maildir::MaildirSessionSync, Result};

use super::Flags;

use super::SetFlags;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagsError(#[source] maildirpp::Error, String, String, Flags),
}

#[derive(Clone)]
pub struct SetFlagsMaildir {
    session: MaildirSessionSync,
}

impl SetFlagsMaildir {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn SetFlags>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl SetFlags for SetFlagsMaildir {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("maildir: setting flag(s) {flags} to envelope {id} from folder {folder}");

        let session = self.session.lock().await;
        let mdir = session.get_mdir_from_dir(folder)?;

        id.iter().try_for_each(|ref id| {
            mdir.set_flags(id, &flags.to_mdir_string()).map_err(|err| {
                Error::SetFlagsError(err, folder.to_owned(), id.to_string(), flags.to_owned())
            })
        })?;

        Ok(())
    }
}
