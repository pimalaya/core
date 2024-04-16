use crate::{debug, info};
use async_trait::async_trait;
use utf7_imap::encode_utf7_imap as encode_utf7;

use crate::{folder::error::Error, imap::ImapContextSync, AnyResult};

use super::AddFolder;

#[derive(Clone, Debug)]
pub struct AddImapFolder {
    ctx: ImapContextSync,
}

impl AddImapFolder {
    pub fn new(ctx: &ImapContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &ImapContextSync) -> Box<dyn AddFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &ImapContextSync) -> Option<Box<dyn AddFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl AddFolder for AddImapFolder {
    async fn add_folder(&self, folder: &str) -> AnyResult<()> {
        info!("creating imap folder {folder}");

        let mut ctx = self.ctx.lock().await;
        let config = &ctx.account_config;

        let folder = config.get_folder_alias(folder);
        let folder_encoded = encode_utf7(folder.clone());
        debug!("utf7 encoded folder: {folder_encoded}");

        ctx.exec(
            |session| session.create(&folder_encoded),
            |err| Error::CreateFolderImapError(err, folder.clone()),
        )
        .await?;

        Ok(())
    }
}
