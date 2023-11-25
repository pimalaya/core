use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    email::{envelope::Id, Flags},
    imap::ImapSessionSync,
    Result,
};

use super::SetFlags;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot set flags {3} to envelope(s) {2} from folder {1}")]
    SetFlagError(#[source] imap::Error, String, Id, Flags),
}

#[derive(Clone, Debug)]
pub struct SetFlagsImap {
    session: ImapSessionSync,
}

impl SetFlagsImap {
    pub fn new(session: &ImapSessionSync) -> Box<dyn SetFlags> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl SetFlags for SetFlagsImap {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("setting imap flag(s) {flags} to envelope {id} from folder {folder}");

        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
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
                    let query = format!("FLAGS ({})", flags.to_imap_query_string());
                    session.uid_store(id.join(","), query)
                },
                |err| Error::SetFlagError(err, folder.clone(), id.clone(), flags.clone()).into(),
            )
            .await?;

        Ok(())
    }
}
