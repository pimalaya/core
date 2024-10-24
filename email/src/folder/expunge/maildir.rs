use async_trait::async_trait;
use tracing::info;

use super::ExpungeFolder;
use crate::{folder::error::Error, maildir::MaildirContextSync, AnyResult};

pub struct ExpungeMaildirFolder {
    ctx: MaildirContextSync,
}

impl ExpungeMaildirFolder {
    pub fn new(ctx: &MaildirContextSync) -> Self {
        Self { ctx: ctx.clone() }
    }

    pub fn new_boxed(ctx: &MaildirContextSync) -> Box<dyn ExpungeFolder> {
        Box::new(Self::new(ctx))
    }

    pub fn some_new_boxed(ctx: &MaildirContextSync) -> Option<Box<dyn ExpungeFolder>> {
        Some(Self::new_boxed(ctx))
    }
}

#[async_trait]
impl ExpungeFolder for ExpungeMaildirFolder {
    async fn expunge_folder(&self, folder: &str) -> AnyResult<()> {
        info!("expunging maildir folder {folder}");

        let ctx = self.ctx.lock().await;
        let mdir = ctx.get_maildir_from_folder_alias(folder)?;

        let entries = mdir
            .read()
            .map_err(|err| Error::ListCurrentFolderMaildirError(err, mdir.path().to_owned()))?;

        entries
            .filter(|entry| entry.has_trash_flag())
            .try_for_each(|entry| {
                entry
                    .remove()
                    .map_err(|err| Error::RemoveMaildirEntryError(err, entry.path().to_owned()))
            })?;

        Ok(())
    }
}
