use async_trait::async_trait;
use log::info;
use thiserror::Error;

use crate::{envelope::SingleId, maildir::MaildirSessionSync, Result};

use super::{AddMessage, Flags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot add maildir message to folder {1} with flags {2}")]
    StoreWithFlagsError(#[source] maildirpp::Error, String, Flags),
}

#[derive(Clone)]
pub struct AddMaildirMessage {
    session: MaildirSessionSync,
}

impl AddMaildirMessage {
    pub fn new(session: &MaildirSessionSync) -> Option<Box<dyn AddMessage>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl AddMessage for AddMaildirMessage {
    async fn add_message_with_flags(
        &self,
        folder: &str,
        raw_msg: &[u8],
        flags: &Flags,
    ) -> Result<SingleId> {
        info!("adding raw email message to folder {folder} with flags {flags}");

        let session = self.session.lock().await;
        let mdir = session.get_maildir_from_folder_name(folder)?;

        let id = mdir
            .store_cur_with_flags(raw_msg, &flags.to_mdir_string())
            .map_err(|err| Error::StoreWithFlagsError(err, folder.to_owned(), flags.clone()))?;

        Ok(SingleId::from(id))
    }
}
