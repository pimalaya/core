use std::error;

use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{
    email::{envelope::Id, Flag},
    imap::ImapSessionSync,
    Result,
};

use super::AddFlag;

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot add flag {3} to envelope(s) {2} from folder {1}")]
    AddFlagError(#[source] imap::Error, String, Id, Flag),
}

impl Error {
    pub fn select_folder(err: imap::Error, folder: String) -> Box<dyn error::Error + Send> {
        Box::new(Self::SelectFolderError(err, folder))
    }

    pub fn add_flag(
        err: imap::Error,
        folder: String,
        id: Id,
        flag: Flag,
    ) -> Box<dyn error::Error + Send> {
        Box::new(Self::AddFlagError(err, folder, id, flag))
    }
}

#[derive(Clone, Debug)]
pub struct AddImapFlag {
    session: ImapSessionSync,
}

impl AddImapFlag {
    pub fn new(session: &ImapSessionSync) -> Box<dyn AddFlag> {
        let session = session.clone();
        Box::new(Self { session })
    }
}

#[async_trait]
impl AddFlag for AddImapFlag {
    async fn add_flag(&mut self, folder: &str, id: Id, flag: Flag) -> Result<()> {
        info!("adding flag {flag} to imap envelope {id} from folder {folder}");

        let uids = id.join(",");
        let mut session = self.session.lock().await;

        let folder = session.account_config.get_folder_alias(folder)?;
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        session
            .execute(
                |session| session.select(&folder_encoded),
                |err| Error::select_folder(err, folder.clone()),
            )
            .await?;

        session
            .execute(
                |session| {
                    let query = format!("+FLAGS ({})", flag.to_imap_query_string());
                    session.uid_store(&uids, query)
                },
                |err| Error::add_flag(err, folder.clone(), id.clone(), flag.clone()),
            )
            .await?;

        Ok(())
    }
}
