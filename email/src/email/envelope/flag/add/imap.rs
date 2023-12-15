use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, imap::ImapSessionSync, Result};

use super::{AddFlags, Flags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot add flags {3} to envelope(s) {2} from folder {1}")]
    AddFlagError(#[source] imap::Error, String, Id, Flags),
}

#[derive(Clone, Debug)]
pub struct AddFlagsImap {
    session: ImapSessionSync,
}

impl AddFlagsImap {
    pub fn new(session: &ImapSessionSync) -> Option<Box<dyn AddFlags>> {
        let session = session.clone();
        Some(Box::new(Self { session }))
    }
}

#[async_trait]
impl AddFlags for AddFlagsImap {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("adding imap flag(s) {flags} to envelope {id} from folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(
                |session| session.select(&folder_encoded),
                |err| Error::SelectFolderError(err, folder.clone()).into(),
            )
            .await?;

        session
            .execute(
                |session| {
                    let query = format!("+FLAGS ({})", flags.to_imap_query_string());
                    session.uid_store(id.join(","), query)
                },
                |err| Error::AddFlagError(err, folder.clone(), id.clone(), flags.clone()).into(),
            )
            .await?;

        Ok(())
    }
}
