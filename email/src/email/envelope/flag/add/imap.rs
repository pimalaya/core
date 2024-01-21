use async_trait::async_trait;
use log::{debug, info};
use thiserror::Error;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{envelope::Id, imap::ImapContextSync, Result};

use super::{AddFlags, Flags};

#[derive(Error, Debug)]
pub enum Error {
    #[error("cannot select imap folder {1}")]
    SelectFolderError(#[source] imap::Error, String),
    #[error("cannot add imap flags {3} to envelope(s) {2} from folder {1}")]
    AddFlagError(#[source] imap::Error, String, Id, Flags),
}

#[derive(Clone, Debug)]
pub struct AddImapFlags {
    ctx: ImapContextSync,
}

impl AddImapFlags {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn AddFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn AddFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddFlags for AddImapFlags {
    async fn add_flags(&self, folder: &str, id: &Id, flags: &Flags) -> Result<()> {
        info!("adding imap flag(s) {flags} to envelope {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderError(err, folder.clone()).into(),
        )
        .await?;

        ctx.exec(
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
