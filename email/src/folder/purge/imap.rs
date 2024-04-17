use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use super::PurgeFolder;
use crate::{
    debug,
    flag::{Flag, Flags},
    folder::error::Error,
    imap::ImapContextSync,
    info, AnyResult,
};

#[derive(Debug)]
pub struct PurgeImapFolder {
    ctx: ImapContextSync,
}

impl PurgeImapFolder {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn PurgeFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn PurgeFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl PurgeFolder for PurgeImapFolder {
    async fn purge_folder(&self, folder: &str) -> AnyResult<()> {
        info!("purging imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        let flags = Flags::from_iter([Flag::Deleted]);
        let uids = String::from("1:*");

        ctx.exec(
            |session| session.select(&folder_encoded),
            |err| Error::SelectFolderImapError(err, folder.clone()),
        )
        .await?;

        ctx.exec(
            |session| {
                session.uid_store(&uids, format!("+FLAGS ({})", flags.to_imap_query_string()))
            },
            |err| Error::AddDeletedFlagImapError(err, folder.clone()),
        )
        .await?;

        ctx.exec(
            |session| session.expunge(),
            |err| Error::ExpungeFolderImapError(err, folder.clone()),
        )
        .await?;

        Ok(())
    }
}
