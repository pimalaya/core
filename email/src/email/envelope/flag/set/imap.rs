use async_trait::async_trait;
use log::{debug, info};
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{email::error::Error, envelope::Id, imap::ImapContextSync};

use super::{Flags, SetFlags};

#[derive(Clone, Debug)]
pub struct SetImapFlags {
    ctx: ImapContextSync,
}

impl SetImapFlags {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn SetFlags> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn SetFlags>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl SetFlags for SetImapFlags {
    async fn set_flags(&self, folder: &str, id: &Id, flags: &Flags) -> crate::Result<()> {
        info!("setting imap flag(s) {flags} to envelope {id} from folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()).into(),
        )
        .await?;

        ctx.exec(
            |session| {
                let query = format!("FLAGS ({})", flags.to_imap_query_string());
                session.uid_store(id.join(","), query)
            },
            |err| Error::SetFlagImapError(err, folder.clone(), id.clone(), flags.clone()).into(),
        )
        .await?;

        Ok(())
    }
}
